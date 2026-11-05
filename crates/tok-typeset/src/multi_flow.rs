//! Multi-flow Constraint Solver for Talmud (ש"ס) and Mikraot Gedolot.
//!
//! Synchronizes 3-8 parallel commentary flows on the same page/spread,
//! using spread-level slack bounding (2%-5%) to contain spillover cascades.
//! Supports:
//! - Recto / Verso Facing Spreads (Rashi on inner spine margin, Tosafot on outer margin).
//! - Dynamic L-Shaped commentary expansion below early-terminating Gemara text ("צורת הדף").
//! - Floating Footnotes with bottom-page height reservation and gutter separation.

use crate::solver::{
    ColumnSpec, ColumnWidthDistribution, ConstraintStrength, GeometricConstraintSolver,
    MultiColumnConstraintModel, PageConstraintModel, PageGeometryConfig,
};
use tok_core::FlowId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpreadSide {
    /// Right-hand page in Hebrew RTL book (odd page: 1, 3, 5...). Spine is on the LEFT.
    Recto,
    /// Left-hand page in Hebrew RTL book (even page: 2, 4, 6...). Spine is on the RIGHT.
    Verso,
}

/// Placement role of a text stream on a page layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlowPlacementRole {
    /// Primary content stream (central column in Talmud/classic, or leading column in dual-stream).
    #[default]
    Primary,
    /// Commentary along the inner spine edge (left on Recto, right on Verso in RTL).
    InnerSpine,
    /// Commentary along the outer margin edge (right on Recto, left on Verso in RTL).
    OuterMargin,
    /// Footnote or apparatus band across the bottom of the page.
    BottomBand,
    /// Arbitrary parallel column index (0-based across physical spread).
    Column(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlowGeometrySpec {
    pub flow_id: FlowId,
    pub priority: u8,
    pub role: FlowPlacementRole,
    pub width_ratio: Option<f32>,
    pub min_width_pt: f32,
    pub max_width_pt: f32,
    pub target_height_pt: f32,
}

impl FlowGeometrySpec {
    pub fn new(flow_id: FlowId, priority: u8) -> Self {
        let role = match priority {
            1 => FlowPlacementRole::Primary,
            2 => FlowPlacementRole::InnerSpine,
            3 => FlowPlacementRole::OuterMargin,
            _ => FlowPlacementRole::Primary,
        };
        Self {
            flow_id,
            priority,
            role,
            width_ratio: None,
            min_width_pt: 50.0,
            max_width_pt: 1000.0,
            target_height_pt: 500.0,
        }
    }

    pub fn with_role(mut self, role: FlowPlacementRole) -> Self {
        self.role = role;
        self
    }

    pub fn with_width_ratio(mut self, ratio: f32) -> Self {
        self.width_ratio = Some(ratio);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SolvedFlowAllocation {
    pub flow_id: FlowId,
    pub allocated_x_pt: f32,
    pub allocated_y_pt: f32,
    pub allocated_width_pt: f32,
    pub allocated_height_pt: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DynamicTalmudPageResult {
    pub allocations: Vec<SolvedFlowAllocation>,
    pub footnote_allocation: Option<SolvedFlowAllocation>,
    pub gemara_height_pt: f32,
    pub primary_height_pt: f32,
    pub has_l_shape_expansion: bool,
}

pub type DynamicPageResult = DynamicTalmudPageResult;

/// Column a flow is placed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FlowRole {
    Main,
    Rashi,
    Tosafot,
}

impl FlowRole {
    fn of(spec: &FlowGeometrySpec) -> Self {
        match spec.role {
            FlowPlacementRole::Primary => FlowRole::Main,
            FlowPlacementRole::InnerSpine => FlowRole::Rashi,
            FlowPlacementRole::OuterMargin => FlowRole::Tosafot,
            _ => {
                if spec.priority == 1 || spec.flow_id.0 == "main" {
                    FlowRole::Main
                } else if spec.priority == 2 {
                    FlowRole::Rashi
                } else {
                    FlowRole::Tosafot
                }
            }
        }
    }
}

/// Clamps a length to a finite, non-negative value.
fn non_negative(v: f32) -> f32 {
    if v.is_finite() {
        v.max(0.0)
    } else {
        0.0
    }
}

pub struct MultiFlowSolver;

impl MultiFlowSolver {
    pub const MAX_ITERATIONS: usize = 1000;
    pub const DEFAULT_GUTTER_PT: f32 = 12.0;
    /// Minimum gap between Gemara bottom and the free space that triggers the L-shape.
    pub const L_SHAPE_MIN_GAP_PT: f32 = 30.0;

    fn margins(margin_inner_pt: f32, margin_outer_pt: f32, side: SpreadSide) -> (f32, f32) {
        match side {
            SpreadSide::Recto => (margin_inner_pt, margin_outer_pt),
            SpreadSide::Verso => (margin_outer_pt, margin_inner_pt),
        }
    }

    /// Gutter between columns, shrunk on pages too narrow for the default.
    fn gutter_for(content_width: f32) -> f32 {
        Self::DEFAULT_GUTTER_PT.min(content_width * 0.05)
    }

    /// Solves the geometric partitioning for a classic Talmudic page layout:
    /// Center: Main Gemara text
    /// Inner column (Right in RTL spread): Rashi commentary
    /// Outer column (Left in RTL spread): Tosafot commentary
    pub fn solve_talmud_page(
        page_width_pt: f32,
        page_height_pt: f32,
        margin_x_pt: f32,
        margin_y_pt: f32,
        flows: &[FlowGeometrySpec],
        _slack_ratio: f32,
    ) -> Vec<SolvedFlowAllocation> {
        Self::solve_talmud_spread(
            page_width_pt,
            page_height_pt,
            margin_x_pt,
            margin_x_pt,
            margin_y_pt,
            flows,
            SpreadSide::Verso, // Default legacy orientation
        )
    }

    /// Solves the geometric partitioning for facing spreads (Recto vs Verso):
    /// In Hebrew typography (reading right to left):
    /// - Recto (Right page): Spine is on the LEFT.
    ///   Inner margin = Left (Rashi), Outer margin = Right (Tosafot).
    /// - Verso (Left page): Spine is on the RIGHT.
    ///   Inner margin = Right (Rashi), Outer margin = Left (Tosafot).
    pub fn solve_talmud_spread(
        page_width_pt: f32,
        page_height_pt: f32,
        margin_inner_pt: f32,
        margin_outer_pt: f32,
        margin_y_pt: f32,
        flows: &[FlowGeometrySpec],
        side: SpreadSide,
    ) -> Vec<SolvedFlowAllocation> {
        Self::solve_generic_spread(
            page_width_pt,
            page_height_pt,
            margin_inner_pt,
            margin_outer_pt,
            margin_y_pt,
            flows,
            side,
        )
    }

    /// Determines geometric ordering and proportions for column streams.
    fn order_flows_and_proportions(
        flows: &[FlowGeometrySpec],
        side: SpreadSide,
        _content_width: f32,
    ) -> (Vec<&FlowGeometrySpec>, ColumnWidthDistribution) {
        let num_cols = flows.len();

        // Check for custom width ratios
        if flows.iter().any(|f| f.width_ratio.is_some()) {
            let ratios: Vec<f32> = flows
                .iter()
                .map(|f| f.width_ratio.unwrap_or(1.0 / num_cols as f32))
                .collect();
            return (
                flows.iter().collect(),
                ColumnWidthDistribution::Proportional(ratios),
            );
        }

        // 2-stream: (Primary 60%, Secondary 40%)
        if num_cols == 2 {
            let primary_idx = flows
                .iter()
                .position(|f| f.role == FlowPlacementRole::Primary || f.priority == 1)
                .unwrap_or(0);
            let secondary_idx = if primary_idx == 0 { 1 } else { 0 };

            let is_inner_secondary = flows[secondary_idx].role == FlowPlacementRole::InnerSpine
                || flows[secondary_idx].priority == 2;

            let (left_idx, right_idx, left_ratio, right_ratio) = match (side, is_inner_secondary) {
                (SpreadSide::Recto, true) | (SpreadSide::Verso, false) => {
                    (secondary_idx, primary_idx, 0.40, 0.60)
                }
                _ => (primary_idx, secondary_idx, 0.60, 0.40),
            };

            let ordered = vec![&flows[left_idx], &flows[right_idx]];
            let dist = ColumnWidthDistribution::Proportional(vec![left_ratio, right_ratio]);
            return (ordered, dist);
        }

        // 3-stream: Talmud layout (Main 40%, Inner 28%, Outer 32%)
        if num_cols == 3 {
            let main_idx = flows
                .iter()
                .position(|f| FlowRole::of(f) == FlowRole::Main)
                .unwrap_or(0);
            let rashi_idx = flows
                .iter()
                .position(|f| FlowRole::of(f) == FlowRole::Rashi)
                .unwrap_or(1);
            let tosafot_idx = flows
                .iter()
                .position(|f| FlowRole::of(f) == FlowRole::Tosafot)
                .unwrap_or(2);

            let (left_idx, right_idx, left_ratio, right_ratio) = match side {
                SpreadSide::Verso => (tosafot_idx, rashi_idx, 0.32, 0.28),
                SpreadSide::Recto => (rashi_idx, tosafot_idx, 0.28, 0.32),
            };

            let ordered = vec![&flows[left_idx], &flows[main_idx], &flows[right_idx]];
            let dist = ColumnWidthDistribution::Proportional(vec![left_ratio, 0.40, right_ratio]);
            return (ordered, dist);
        }

        // 4-stream: Mikraot Gedolot / 4 columns
        if num_cols == 4 {
            let ratios = match side {
                SpreadSide::Recto => vec![0.24, 0.36, 0.22, 0.18],
                SpreadSide::Verso => vec![0.18, 0.22, 0.36, 0.24],
            };
            return (
                flows.iter().collect(),
                ColumnWidthDistribution::Proportional(ratios),
            );
        }

        // Generic N columns: equal distribution
        (flows.iter().collect(), ColumnWidthDistribution::Equal)
    }

    /// Solves layout allocation across a spread for an arbitrary number of flows (1, 2, 3, 4, N)
    /// using constraint specifications, roles, and width ratios.
    pub fn solve_generic_spread(
        page_width_pt: f32,
        page_height_pt: f32,
        margin_inner_pt: f32,
        margin_outer_pt: f32,
        margin_y_pt: f32,
        flows: &[FlowGeometrySpec],
        side: SpreadSide,
    ) -> Vec<SolvedFlowAllocation> {
        if flows.is_empty() {
            return Vec::new();
        }

        let (margin_left, margin_right) = Self::margins(margin_inner_pt, margin_outer_pt, side);
        let content_width = non_negative(page_width_pt - margin_left - margin_right);
        let content_height = non_negative(page_height_pt - 2.0 * margin_y_pt);

        if content_width <= 0.0
            || content_height <= 0.0
            || !content_width.is_finite()
            || !content_height.is_finite()
        {
            return flows
                .iter()
                .map(|f| SolvedFlowAllocation {
                    flow_id: f.flow_id.clone(),
                    allocated_x_pt: margin_left.max(0.0),
                    allocated_y_pt: margin_y_pt.max(0.0),
                    allocated_width_pt: 0.0,
                    allocated_height_pt: 0.0,
                })
                .collect();
        }

        let gutter = Self::gutter_for(content_width);

        // Separate column flows from bottom-band apparatus flows (e.g. footnotes/Ein Mishpat).
        // Bottom bands span full content width across the page and must not steal horizontal column measure.
        let col_flows: Vec<FlowGeometrySpec> = flows
            .iter()
            .filter(|f| f.role != FlowPlacementRole::BottomBand)
            .cloned()
            .collect();
        let bottom_flows: Vec<FlowGeometrySpec> = flows
            .iter()
            .filter(|f| f.role == FlowPlacementRole::BottomBand)
            .cloned()
            .collect();

        let mut bottom_allocs: Vec<SolvedFlowAllocation> = bottom_flows
            .into_iter()
            .map(|f| SolvedFlowAllocation {
                flow_id: f.flow_id,
                allocated_x_pt: margin_left,
                allocated_y_pt: margin_y_pt + content_height,
                allocated_width_pt: content_width,
                allocated_height_pt: 0.0,
            })
            .collect();

        if col_flows.is_empty() {
            return bottom_allocs;
        }

        let num_cols = col_flows.len();

        if num_cols == 1 {
            let mut res = vec![SolvedFlowAllocation {
                flow_id: col_flows[0].flow_id.clone(),
                allocated_x_pt: margin_left,
                allocated_y_pt: margin_y_pt,
                allocated_width_pt: content_width,
                allocated_height_pt: content_height,
            }];
            res.append(&mut bottom_allocs);
            return res;
        }

        let (ordered_flows, proportions) =
            Self::order_flows_and_proportions(&col_flows, side, content_width);

        let mut solver = GeometricConstraintSolver::new();
        let page_model = PageConstraintModel::new();
        let config = PageGeometryConfig::new(
            page_width_pt,
            page_height_pt,
            margin_inner_pt,
            margin_outer_pt,
            margin_y_pt,
            margin_y_pt,
            side,
        );

        if page_model.apply(&mut solver, &config).is_err() {
            return flows
                .iter()
                .map(|f| SolvedFlowAllocation {
                    flow_id: f.flow_id.clone(),
                    allocated_x_pt: margin_left.max(0.0),
                    allocated_y_pt: margin_y_pt.max(0.0),
                    allocated_width_pt: 0.0,
                    allocated_height_pt: 0.0,
                })
                .collect();
        }

        let flow_ids: Vec<FlowId> = ordered_flows.iter().map(|f| f.flow_id.clone()).collect();
        let col_model = MultiColumnConstraintModel::new(&flow_ids);
        let col_specs: Vec<ColumnSpec> = ordered_flows
            .iter()
            .map(|f| {
                ColumnSpec::new(f.flow_id.clone())
                    .with_min_width(f.min_width_pt.min(content_width / num_cols as f32))
                    .with_max_width(f.max_width_pt)
            })
            .collect();

        if col_model
            .apply(&mut solver, &page_model, gutter, &col_specs)
            .is_err()
        {
            return flows
                .iter()
                .map(|f| SolvedFlowAllocation {
                    flow_id: f.flow_id.clone(),
                    allocated_x_pt: margin_left.max(0.0),
                    allocated_y_pt: margin_y_pt.max(0.0),
                    allocated_width_pt: 0.0,
                    allocated_height_pt: 0.0,
                })
                .collect();
        }

        let _ = proportions.apply(
            &mut solver,
            &col_model.columns,
            ConstraintStrength::Required,
        );

        let solved_cols = col_model.get_allocations(&solver);
        let allocations: Vec<SolvedFlowAllocation> = solved_cols
            .into_iter()
            .map(|sc| SolvedFlowAllocation {
                flow_id: sc.flow_id,
                allocated_x_pt: sc.rect.x,
                allocated_y_pt: sc.rect.y,
                allocated_width_pt: sc.rect.width,
                allocated_height_pt: sc.rect.height,
            })
            .collect();

        let mut final_allocations = allocations;
        final_allocations.append(&mut bottom_allocs);

        if flows.len() == 2 {
            let mut matched = vec![final_allocations[0].clone(), final_allocations[1].clone()];
            for a in &final_allocations {
                if a.flow_id == flows[0].flow_id {
                    matched[0] = a.clone();
                } else if a.flow_id == flows[1].flow_id {
                    matched[1] = a.clone();
                }
            }
            return matched;
        }

        final_allocations
    }

    /// Solves dynamic Talmud layout with L-shaped commentary expansion below Gemara
    /// and bottom-anchored floating footnotes.
    #[allow(clippy::too_many_arguments)]
    pub fn solve_talmud_dynamic_with_footnotes(
        page_width_pt: f32,
        page_height_pt: f32,
        margin_inner_pt: f32,
        margin_outer_pt: f32,
        margin_y_pt: f32,
        flows: &[FlowGeometrySpec],
        side: SpreadSide,
        gemara_target_height_pt: Option<f32>,
        footnote_target_height_pt: Option<f32>,
    ) -> DynamicTalmudPageResult {
        Self::solve_dynamic_spread_with_footnotes(
            page_width_pt,
            page_height_pt,
            margin_inner_pt,
            margin_outer_pt,
            margin_y_pt,
            flows,
            side,
            gemara_target_height_pt,
            footnote_target_height_pt,
            None,
        )
    }

    /// Solves dynamic layout with L-shaped commentary expansion below primary stream
    /// and bottom-anchored floating footnotes for arbitrary flows.
    #[allow(clippy::too_many_arguments)]
    pub fn solve_dynamic_spread_with_footnotes(
        page_width_pt: f32,
        page_height_pt: f32,
        margin_inner_pt: f32,
        margin_outer_pt: f32,
        margin_y_pt: f32,
        flows: &[FlowGeometrySpec],
        side: SpreadSide,
        primary_target_height_pt: Option<f32>,
        footnote_target_height_pt: Option<f32>,
        expansion_flow_id: Option<&FlowId>,
    ) -> DynamicTalmudPageResult {
        Self::solve_dynamic_spread_with_commentary_absorption(
            page_width_pt,
            page_height_pt,
            margin_inner_pt,
            margin_outer_pt,
            margin_y_pt,
            flows,
            side,
            primary_target_height_pt,
            footnote_target_height_pt,
            expansion_flow_id,
            None,
        )
    }

    /// Solves dynamic layout with L-shaped commentary expansion below early-terminating primary
    /// or commentary streams, and bottom-anchored floating footnotes.
    #[allow(clippy::too_many_arguments)]
    pub fn solve_dynamic_spread_with_commentary_absorption(
        page_width_pt: f32,
        page_height_pt: f32,
        margin_inner_pt: f32,
        margin_outer_pt: f32,
        margin_y_pt: f32,
        flows: &[FlowGeometrySpec],
        side: SpreadSide,
        primary_target_height_pt: Option<f32>,
        footnote_target_height_pt: Option<f32>,
        expansion_flow_id: Option<&FlowId>,
        commentary_termination: Option<(FlowId, f32)>,
    ) -> DynamicTalmudPageResult {
        let (margin_left, margin_right) = Self::margins(margin_inner_pt, margin_outer_pt, side);

        let content_width = non_negative(page_width_pt - margin_left - margin_right);
        let mut available_height = non_negative(page_height_pt - 2.0 * margin_y_pt);
        let gutter = Self::gutter_for(content_width);

        // 1. Allocate Floating Footnotes at bottom
        let footnote_flow = flows
            .iter()
            .find(|f| f.role == FlowPlacementRole::BottomBand || f.flow_id.0 == "footnote");

        let footnote_target =
            footnote_target_height_pt.or_else(|| footnote_flow.map(|f| f.target_height_pt));

        let footnote_allocation = match footnote_target.map(non_negative) {
            Some(fn_height) if fn_height > 0.0 && available_height > 0.0 => {
                let actual_fn_height = fn_height.min(available_height * 0.40);
                let fn_y = margin_y_pt + available_height - actual_fn_height;
                available_height = non_negative(available_height - actual_fn_height - gutter);

                let fn_id = footnote_flow
                    .map(|f| f.flow_id.clone())
                    .unwrap_or_else(|| FlowId::new("footnote"));

                Some(SolvedFlowAllocation {
                    flow_id: fn_id,
                    allocated_x_pt: margin_left,
                    allocated_y_pt: fn_y,
                    allocated_width_pt: content_width,
                    allocated_height_pt: actual_fn_height,
                })
            }
            _ => None,
        };

        // 2. Compute column allocation (excluding bottom band)
        let column_flows: Vec<FlowGeometrySpec> = flows
            .iter()
            .filter(|f| f.role != FlowPlacementRole::BottomBand)
            .cloned()
            .collect();

        let mut allocations = Self::solve_generic_spread(
            page_width_pt,
            available_height + 2.0 * margin_y_pt,
            margin_inner_pt,
            margin_outer_pt,
            margin_y_pt,
            &column_flows,
            side,
        );

        // 3. Dynamic L-Shaped expansion if primary stream ends before bottom and
        //    there is a commentary flow to expand into the freed space.
        let primary_index = column_flows.iter().position(|f| {
            f.role == FlowPlacementRole::Primary
                || f.priority == 1
                || FlowRole::of(f) == FlowRole::Main
        });

        let commentary_spec = if let Some(exp_id) = expansion_flow_id {
            column_flows.iter().find(|f| &f.flow_id == exp_id)
        } else {
            column_flows
                .iter()
                .find(|f| {
                    f.role == FlowPlacementRole::InnerSpine
                        || f.priority == 2
                        || FlowRole::of(f) == FlowRole::Rashi
                })
                .or_else(|| {
                    column_flows
                        .iter()
                        .enumerate()
                        .find(|(idx, _)| Some(*idx) != primary_index)
                        .map(|(_, f)| f)
                })
        };

        let has_commentary = commentary_spec.is_some() && column_flows.len() > 1;
        let mut has_l_shape = false;
        let primary_h = match primary_target_height_pt {
            Some(target_h) => {
                let actual_primary_h = non_negative(target_h).min(available_height);
                if actual_primary_h < available_height - Self::L_SHAPE_MIN_GAP_PT && has_commentary
                {
                    if let (Some(primary_idx), Some(comm_spec)) = (primary_index, commentary_spec) {
                        let primary_flow_id = &column_flows[primary_idx].flow_id;
                        if let Some(primary_alloc) = allocations
                            .iter_mut()
                            .find(|a| &a.flow_id == primary_flow_id)
                        {
                            primary_alloc.allocated_height_pt = actual_primary_h;
                            let (gx, gw) = (
                                primary_alloc.allocated_x_pt,
                                primary_alloc.allocated_width_pt,
                            );

                            let expansion_y = margin_y_pt + actual_primary_h + gutter;
                            let expansion_h =
                                non_negative(available_height - actual_primary_h - gutter);

                            let exp_id = if comm_spec.flow_id.0 == "rashi" {
                                FlowId::new("rashi_expansion")
                            } else {
                                FlowId::new(format!("{}_expansion", comm_spec.flow_id.0))
                            };

                            allocations.push(SolvedFlowAllocation {
                                flow_id: exp_id,
                                allocated_x_pt: gx,
                                allocated_y_pt: expansion_y,
                                allocated_width_pt: gw,
                                allocated_height_pt: expansion_h,
                            });
                            has_l_shape = true;
                        }
                    }
                }
                actual_primary_h
            }
            None => available_height,
        };

        // 4. Dynamic commentary absorption if a commentary finishes early and primary does not expand
        if !has_l_shape {
            if let Some((ref term_id, term_h)) = commentary_termination {
                let actual_term_h = non_negative(term_h).min(available_height);
                if actual_term_h < available_height - Self::L_SHAPE_MIN_GAP_PT {
                    if let Some(term_alloc) = allocations.iter_mut().find(|a| &a.flow_id == term_id)
                    {
                        term_alloc.allocated_height_pt = actual_term_h;
                        let (tx, tw) = (term_alloc.allocated_x_pt, term_alloc.allocated_width_pt);
                        let expansion_y = margin_y_pt + actual_term_h + gutter;
                        let expansion_h = non_negative(available_height - actual_term_h - gutter);

                        // Find absorbing commentary (must be distinct from the terminating flow)
                        let absorbing_flow = if let Some(exp_id) = expansion_flow_id {
                            if exp_id != term_id {
                                column_flows.iter().find(|f| &f.flow_id == exp_id)
                            } else {
                                column_flows.iter().find(|f| {
                                    &f.flow_id != term_id && f.role != FlowPlacementRole::Primary
                                })
                            }
                        } else {
                            column_flows.iter().find(|f| {
                                &f.flow_id != term_id && f.role != FlowPlacementRole::Primary
                            })
                        };

                        if let Some(absorb_spec) = absorbing_flow {
                            let exp_id =
                                FlowId::new(format!("{}_expansion", absorb_spec.flow_id.0));
                            allocations.push(SolvedFlowAllocation {
                                flow_id: exp_id,
                                allocated_x_pt: tx,
                                allocated_y_pt: expansion_y,
                                allocated_width_pt: tw,
                                allocated_height_pt: expansion_h,
                            });
                            has_l_shape = true;
                        }
                    }
                }
            }
        }

        DynamicTalmudPageResult {
            allocations,
            footnote_allocation,
            gemara_height_pt: primary_h,
            primary_height_pt: primary_h,
            has_l_shape_expansion: has_l_shape,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(id: &str, priority: u8) -> FlowGeometrySpec {
        FlowGeometrySpec::new(FlowId::new(id), priority)
    }

    fn talmud_flows() -> Vec<FlowGeometrySpec> {
        vec![spec("main", 1), spec("rashi", 2), spec("tosafot", 3)]
    }

    fn overlaps(a: &SolvedFlowAllocation, b: &SolvedFlowAllocation) -> bool {
        let eps = 1e-3;
        a.allocated_x_pt + eps < b.allocated_x_pt + b.allocated_width_pt
            && b.allocated_x_pt + eps < a.allocated_x_pt + a.allocated_width_pt
            && a.allocated_y_pt + eps < b.allocated_y_pt + b.allocated_height_pt
            && b.allocated_y_pt + eps < a.allocated_y_pt + a.allocated_height_pt
    }

    #[test]
    fn test_multi_flow_partitioning() {
        let flows = talmud_flows();
        let allocs = MultiFlowSolver::solve_talmud_page(595.0, 842.0, 36.0, 36.0, &flows, 0.03);
        assert_eq!(allocs.len(), 3);
        let content_width = 595.0 - 72.0;
        let total_col_width: f32 = allocs.iter().map(|a| a.allocated_width_pt).sum();
        assert!(total_col_width <= content_width);
    }

    #[test]
    fn test_multi_flow_empty_flows() {
        let allocations = MultiFlowSolver::solve_talmud_page(595.0, 842.0, 42.52, 56.69, &[], 0.1);
        assert!(allocations.is_empty());
    }

    #[test]
    fn test_multi_flow_single_flow() {
        let flows = vec![spec("main", 1)];
        let allocations =
            MultiFlowSolver::solve_talmud_page(595.0, 842.0, 42.52, 56.69, &flows, 0.1);
        assert_eq!(allocations.len(), 1);
    }

    #[test]
    fn test_talmud_facing_spreads_recto_verso() {
        let flows = talmud_flows();

        // Recto (Right page): Rashi should be on the LEFT (inner margin near spine)
        let recto = MultiFlowSolver::solve_talmud_spread(
            595.0,
            842.0,
            40.0,
            20.0,
            36.0,
            &flows,
            SpreadSide::Recto,
        );
        let rashi_recto = recto.iter().find(|a| a.flow_id.0 == "rashi").unwrap();
        let tosafot_recto = recto.iter().find(|a| a.flow_id.0 == "tosafot").unwrap();
        assert!(rashi_recto.allocated_x_pt < tosafot_recto.allocated_x_pt);

        // Verso (Left page): Rashi should be on the RIGHT (inner margin near spine)
        let verso = MultiFlowSolver::solve_talmud_spread(
            595.0,
            842.0,
            40.0,
            20.0,
            36.0,
            &flows,
            SpreadSide::Verso,
        );
        let rashi_verso = verso.iter().find(|a| a.flow_id.0 == "rashi").unwrap();
        let tosafot_verso = verso.iter().find(|a| a.flow_id.0 == "tosafot").unwrap();
        assert!(rashi_verso.allocated_x_pt > tosafot_verso.allocated_x_pt);
    }

    #[test]
    fn test_dynamic_talmud_l_shape_and_footnotes() {
        let flows = talmud_flows();

        // Gemara finishes at 300 pt (out of 770 pt available height)
        // Footnote needs 80 pt
        let result = MultiFlowSolver::solve_talmud_dynamic_with_footnotes(
            595.0,
            842.0,
            36.0,
            36.0,
            36.0,
            &flows,
            SpreadSide::Verso,
            Some(300.0),
            Some(80.0),
        );

        assert!(result.footnote_allocation.is_some());
        let fn_alloc = result.footnote_allocation.unwrap();
        assert_eq!(fn_alloc.allocated_height_pt, 80.0);
        assert!(fn_alloc.allocated_y_pt > 600.0); // Placed at bottom

        assert!(result.has_l_shape_expansion);
        // There should be a rashi_expansion box
        assert!(result
            .allocations
            .iter()
            .any(|a| a.flow_id.0 == "rashi_expansion"));
    }

    /// Regression: the expansion box spanned 25%-75% of the page and overlapped
    /// the full-height side columns.
    #[test]
    fn l_shape_expansion_overlaps_nothing() {
        for side in [SpreadSide::Recto, SpreadSide::Verso] {
            let result = MultiFlowSolver::solve_talmud_dynamic_with_footnotes(
                595.0,
                842.0,
                40.0,
                30.0,
                36.0,
                &talmud_flows(),
                side,
                Some(300.0),
                Some(80.0),
            );
            assert!(result.has_l_shape_expansion);
            let mut all = result.allocations.clone();
            all.extend(result.footnote_allocation.clone());
            for (i, a) in all.iter().enumerate() {
                for b in &all[i + 1..] {
                    assert!(!overlaps(a, b), "{:?} overlaps {:?}", a.flow_id, b.flow_id);
                }
            }
        }
    }

    /// Regression: a main flow identified by priority (not by the id "main")
    /// kept its full height while the expansion was drawn on top of it.
    #[test]
    fn l_shape_shrinks_main_flow_found_by_priority() {
        let flows = vec![spec("gemara", 1), spec("rashi", 2), spec("tosafot", 3)];
        let result = MultiFlowSolver::solve_talmud_dynamic_with_footnotes(
            595.0,
            842.0,
            36.0,
            36.0,
            36.0,
            &flows,
            SpreadSide::Recto,
            Some(200.0),
            None,
        );
        let gemara = result
            .allocations
            .iter()
            .find(|a| a.flow_id.0 == "gemara")
            .unwrap();
        assert_eq!(gemara.allocated_height_pt, 200.0);
        assert!(result.has_l_shape_expansion);
    }

    #[test]
    fn no_expansion_without_commentary() {
        for flows in [vec![], vec![spec("main", 1)]] {
            let result = MultiFlowSolver::solve_talmud_dynamic_with_footnotes(
                595.0,
                842.0,
                36.0,
                36.0,
                36.0,
                &flows,
                SpreadSide::Recto,
                Some(100.0),
                None,
            );
            assert!(!result.has_l_shape_expansion);
            assert!(result
                .allocations
                .iter()
                .all(|a| a.flow_id.0 != "rashi_expansion"));
        }
    }

    #[test]
    fn degenerate_pages_produce_no_negative_sizes() {
        let cases = [
            (100.0, 100.0, 60.0, 60.0, 60.0),
            (20.0, 900.0, 5.0, 5.0, 10.0),
            (595.0, 842.0, 36.0, 36.0, f32::NAN),
        ];
        for (w, h, mi, mo, my) in cases {
            let result = MultiFlowSolver::solve_talmud_dynamic_with_footnotes(
                w,
                h,
                mi,
                mo,
                my,
                &talmud_flows(),
                SpreadSide::Verso,
                Some(-50.0),
                Some(10_000.0),
            );
            for a in result.allocations.iter().chain(&result.footnote_allocation) {
                assert!(a.allocated_width_pt >= 0.0, "{a:?}");
                assert!(a.allocated_height_pt >= 0.0, "{a:?}");
            }
            assert!(result.gemara_height_pt >= 0.0);
        }
        // Narrow page: columns and gutters still fit inside the content box.
        let allocs = MultiFlowSolver::solve_talmud_spread(
            60.0,
            400.0,
            5.0,
            5.0,
            10.0,
            &talmud_flows(),
            SpreadSide::Recto,
        );
        let right = allocs
            .iter()
            .map(|a| a.allocated_x_pt + a.allocated_width_pt)
            .fold(0.0f32, f32::max);
        assert!(right <= 55.0 + 1e-3, "right edge {right}");
    }

    #[test]
    fn test_generic_spread_two_flows() {
        let flows = vec![
            FlowGeometrySpec::new(FlowId("primary".into()), 1)
                .with_role(FlowPlacementRole::Primary),
            FlowGeometrySpec::new(FlowId("commentary".into()), 2)
                .with_role(FlowPlacementRole::InnerSpine),
        ];

        // Recto: InnerSpine should be on the LEFT (spine is left)
        let recto = MultiFlowSolver::solve_generic_spread(
            595.0,
            842.0,
            40.0,
            20.0,
            36.0,
            &flows,
            SpreadSide::Recto,
        );
        assert_eq!(recto.len(), 2);
        let comm_recto = recto.iter().find(|a| a.flow_id.0 == "commentary").unwrap();
        let prim_recto = recto.iter().find(|a| a.flow_id.0 == "primary").unwrap();
        assert!(comm_recto.allocated_x_pt < prim_recto.allocated_x_pt);
        assert!(prim_recto.allocated_width_pt > comm_recto.allocated_width_pt);

        // Verso: InnerSpine should be on the RIGHT (spine is right)
        let verso = MultiFlowSolver::solve_generic_spread(
            595.0,
            842.0,
            40.0,
            20.0,
            36.0,
            &flows,
            SpreadSide::Verso,
        );
        let comm_verso = verso.iter().find(|a| a.flow_id.0 == "commentary").unwrap();
        let prim_verso = verso.iter().find(|a| a.flow_id.0 == "primary").unwrap();
        assert!(comm_verso.allocated_x_pt > prim_verso.allocated_x_pt);
    }

    #[test]
    fn test_generic_spread_four_flows_mikraot_gedolot() {
        let flows = vec![
            spec("torah", 1),
            spec("onkelos", 2),
            spec("rashi", 3),
            spec("ramban", 4),
        ];
        let recto = MultiFlowSolver::solve_generic_spread(
            595.0,
            842.0,
            36.0,
            36.0,
            36.0,
            &flows,
            SpreadSide::Recto,
        );
        assert_eq!(recto.len(), 4);
        for i in 0..3 {
            assert!(
                recto[i].allocated_x_pt + recto[i].allocated_width_pt
                    <= recto[i + 1].allocated_x_pt + 1e-3
            );
        }
    }

    #[test]
    fn test_generic_spread_custom_ratios() {
        let flows = vec![
            FlowGeometrySpec::new(FlowId("f1".into()), 1).with_width_ratio(0.5),
            FlowGeometrySpec::new(FlowId("f2".into()), 2).with_width_ratio(0.25),
            FlowGeometrySpec::new(FlowId("f3".into()), 3).with_width_ratio(0.25),
        ];
        let allocs = MultiFlowSolver::solve_generic_spread(
            600.0,
            800.0,
            30.0,
            30.0,
            30.0,
            &flows,
            SpreadSide::Recto,
        );
        assert_eq!(allocs.len(), 3);
        // f1 should have twice the width of f2 and f3
        assert!((allocs[0].allocated_width_pt - 2.0 * allocs[1].allocated_width_pt).abs() < 1.0);
        assert!((allocs[1].allocated_width_pt - allocs[2].allocated_width_pt).abs() < 1e-3);
    }

    #[test]
    fn test_generic_spread_n_flows_even_distribution() {
        let flows = (1..=5)
            .map(|i| spec(&format!("col_{i}"), i))
            .collect::<Vec<_>>();
        let allocs = MultiFlowSolver::solve_generic_spread(
            600.0,
            800.0,
            30.0,
            30.0,
            30.0,
            &flows,
            SpreadSide::Recto,
        );
        assert_eq!(allocs.len(), 5);
        for i in 0..4 {
            assert!((allocs[i].allocated_width_pt - allocs[i + 1].allocated_width_pt).abs() < 1e-3);
            assert!(
                allocs[i].allocated_x_pt + allocs[i].allocated_width_pt
                    <= allocs[i + 1].allocated_x_pt + 1e-3
            );
        }
    }

    #[test]
    fn test_generalized_l_shape_expansion_custom_flows() {
        let flows = vec![
            FlowGeometrySpec::new(FlowId("main_text".into()), 1)
                .with_role(FlowPlacementRole::Primary),
            FlowGeometrySpec::new(FlowId("commentary".into()), 2)
                .with_role(FlowPlacementRole::InnerSpine),
        ];

        let result = MultiFlowSolver::solve_dynamic_spread_with_footnotes(
            595.0,
            842.0,
            40.0,
            20.0,
            36.0,
            &flows,
            SpreadSide::Recto,
            Some(250.0),
            None,
            None,
        );

        assert!(result.has_l_shape_expansion);
        let main_alloc = result
            .allocations
            .iter()
            .find(|a| a.flow_id.0 == "main_text")
            .unwrap();
        let exp_alloc = result
            .allocations
            .iter()
            .find(|a| a.flow_id.0 == "commentary_expansion")
            .unwrap();
        assert_eq!(main_alloc.allocated_height_pt, 250.0);
        assert_eq!(exp_alloc.allocated_x_pt, main_alloc.allocated_x_pt);
        assert_eq!(exp_alloc.allocated_width_pt, main_alloc.allocated_width_pt);
        assert!(
            exp_alloc.allocated_y_pt > main_alloc.allocated_y_pt + main_alloc.allocated_height_pt
        );

        for (i, a) in result.allocations.iter().enumerate() {
            for b in &result.allocations[i + 1..] {
                assert!(!overlaps(a, b), "{:?} overlaps {:?}", a.flow_id, b.flow_id);
            }
        }
    }

    #[test]
    fn test_generalized_l_shape_with_custom_expansion_id() {
        let flows = vec![
            FlowGeometrySpec::new(FlowId("chumash".into()), 1)
                .with_role(FlowPlacementRole::Primary),
            FlowGeometrySpec::new(FlowId("targum".into()), 2)
                .with_role(FlowPlacementRole::InnerSpine),
            FlowGeometrySpec::new(FlowId("rashi".into()), 3)
                .with_role(FlowPlacementRole::OuterMargin),
        ];

        let exp_id = FlowId("rashi".into());
        let result = MultiFlowSolver::solve_dynamic_spread_with_footnotes(
            595.0,
            842.0,
            36.0,
            36.0,
            36.0,
            &flows,
            SpreadSide::Recto,
            Some(300.0),
            None,
            Some(&exp_id),
        );

        assert!(result.has_l_shape_expansion);
        assert!(result
            .allocations
            .iter()
            .any(|a| a.flow_id.0 == "rashi_expansion"));
        for (i, a) in result.allocations.iter().enumerate() {
            for b in &result.allocations[i + 1..] {
                assert!(!overlaps(a, b), "{:?} overlaps {:?}", a.flow_id, b.flow_id);
            }
        }
    }

    #[test]
    fn test_generalized_l_shape_with_bottom_band_flow() {
        let flows = vec![
            FlowGeometrySpec::new(FlowId("primary".into()), 1)
                .with_role(FlowPlacementRole::Primary),
            FlowGeometrySpec::new(FlowId("perush".into()), 2)
                .with_role(FlowPlacementRole::InnerSpine),
            FlowGeometrySpec::new(FlowId("heorot".into()), 3)
                .with_role(FlowPlacementRole::BottomBand),
        ];

        let result = MultiFlowSolver::solve_dynamic_spread_with_footnotes(
            595.0,
            842.0,
            36.0,
            36.0,
            36.0,
            &flows,
            SpreadSide::Verso,
            Some(200.0),
            Some(100.0),
            None,
        );

        assert!(result.has_l_shape_expansion);
        assert!(result.footnote_allocation.is_some());
        let fn_alloc = result.footnote_allocation.unwrap();
        assert_eq!(fn_alloc.flow_id.0, "heorot");
        assert_eq!(fn_alloc.allocated_height_pt, 100.0);

        let mut all = result.allocations.clone();
        all.push(fn_alloc);
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert!(!overlaps(a, b), "{:?} overlaps {:?}", a.flow_id, b.flow_id);
            }
        }
    }
}
