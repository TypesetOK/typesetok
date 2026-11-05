//! Core Geometric Constraint Solver wrapping Cassowary Simplex.

use crate::solver::model::{
    ConstraintRelation, ConstraintStrength, ConstraintVariable, GeometricConstraint,
    LinearExpression, RectVariables, SolvedRect,
};
use cassowary::WeightedRelation::*;
use cassowary::{AddConstraintError, Expression, Solver};
use std::fmt;

/// Errors arising from constraint satisfaction solving.
#[derive(Debug, Clone, PartialEq)]
pub enum SolverError {
    /// Two or more required constraints are mathematically contradictory.
    Contradiction(String),
    /// Variable was not defined or registered.
    VariableNotFound,
    /// Linear expression contains invalid or NaN coefficients.
    InvalidExpression(String),
    /// Requested geometry is physically degenerated or negative.
    DegenerateGeometry(String),
}

impl fmt::Display for SolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Contradiction(msg) => write!(f, "Geometric constraint contradiction: {}", msg),
            Self::VariableNotFound => write!(f, "Constraint variable not found in solver"),
            Self::InvalidExpression(msg) => write!(f, "Invalid linear expression: {}", msg),
            Self::DegenerateGeometry(msg) => write!(f, "Degenerate geometry: {}", msg),
        }
    }
}

impl std::error::Error for SolverError {}

impl From<AddConstraintError> for SolverError {
    fn from(err: AddConstraintError) -> Self {
        match err {
            AddConstraintError::DuplicateConstraint => {
                Self::Contradiction("Duplicate required constraint".to_string())
            }
            AddConstraintError::UnsatisfiableConstraint => {
                Self::Contradiction("Unsatisfiable required constraint".to_string())
            }
            AddConstraintError::InternalSolverError(msg) => {
                Self::Contradiction(format!("Internal solver error: {}", msg))
            }
        }
    }
}

/// Linear simplex constraint solver for page and column geometry.
pub struct GeometricConstraintSolver {
    solver: Solver,
}

impl GeometricConstraintSolver {
    /// Creates a new empty solver instance.
    pub fn new() -> Self {
        Self {
            solver: Solver::new(),
        }
    }

    /// Converts a domain `LinearExpression` into a `cassowary::Expression`.
    fn convert_expr(expr: &LinearExpression) -> Result<Expression, SolverError> {
        if !expr.constant.is_finite() {
            return Err(SolverError::InvalidExpression(
                "Expression constant must be finite".to_string(),
            ));
        }

        let mut cass_expr = Expression::from_constant(expr.constant);
        for t in &expr.terms {
            if !t.coefficient.is_finite() {
                return Err(SolverError::InvalidExpression(
                    "Expression coefficient must be finite".to_string(),
                ));
            }
            cass_expr = cass_expr + (t.variable.raw * t.coefficient);
        }

        Ok(cass_expr)
    }

    /// Adds a geometric constraint to the system.
    pub fn add_constraint(&mut self, constraint: GeometricConstraint) -> Result<(), SolverError> {
        let diff_expr = constraint.lhs.minus(&constraint.rhs);
        let cass_expr = Self::convert_expr(&diff_expr)?;
        let strength = constraint.strength.to_cassowary();

        let cass_constraint = match constraint.relation {
            ConstraintRelation::Equal => cass_expr | EQ(strength) | 0.0,
            ConstraintRelation::LessThanOrEqual => cass_expr | LE(strength) | 0.0,
            ConstraintRelation::GreaterThanOrEqual => cass_expr | GE(strength) | 0.0,
        };

        self.solver.add_constraint(cass_constraint)?;
        Ok(())
    }

    /// Adds multiple constraints sequentially.
    pub fn add_constraints<I>(&mut self, constraints: I) -> Result<(), SolverError>
    where
        I: IntoIterator<Item = GeometricConstraint>,
    {
        for c in constraints {
            self.add_constraint(c)?;
        }
        Ok(())
    }

    /// Convenience: adds an equality constraint `lhs == rhs`.
    pub fn add_eq(
        &mut self,
        lhs: LinearExpression,
        rhs: LinearExpression,
        strength: ConstraintStrength,
    ) -> Result<(), SolverError> {
        self.add_constraint(GeometricConstraint::eq(lhs, rhs, strength))
    }

    /// Convenience: adds an inequality constraint `lhs <= rhs`.
    pub fn add_le(
        &mut self,
        lhs: LinearExpression,
        rhs: LinearExpression,
        strength: ConstraintStrength,
    ) -> Result<(), SolverError> {
        self.add_constraint(GeometricConstraint::le(lhs, rhs, strength))
    }

    /// Convenience: adds an inequality constraint `lhs >= rhs`.
    pub fn add_ge(
        &mut self,
        lhs: LinearExpression,
        rhs: LinearExpression,
        strength: ConstraintStrength,
    ) -> Result<(), SolverError> {
        self.add_constraint(GeometricConstraint::ge(lhs, rhs, strength))
    }

    /// Constrains a variable to be non-negative: `var >= 0.0`.
    pub fn add_non_negative(&mut self, var: ConstraintVariable) -> Result<(), SolverError> {
        self.add_ge(
            var.to_expr(),
            LinearExpression::constant(0.0),
            ConstraintStrength::Required,
        )
    }

    /// Constrains a variable to be at least `min`.
    pub fn add_min_value(
        &mut self,
        var: ConstraintVariable,
        min: f64,
        strength: ConstraintStrength,
    ) -> Result<(), SolverError> {
        self.add_ge(var.to_expr(), LinearExpression::constant(min), strength)
    }

    /// Constrains a variable to be at most `max`.
    pub fn add_max_value(
        &mut self,
        var: ConstraintVariable,
        max: f64,
        strength: ConstraintStrength,
    ) -> Result<(), SolverError> {
        self.add_le(var.to_expr(), LinearExpression::constant(max), strength)
    }

    /// Sets an edit variable with a given strength.
    pub fn add_edit_variable(
        &mut self,
        var: ConstraintVariable,
        strength: ConstraintStrength,
    ) -> Result<(), SolverError> {
        self.solver
            .add_edit_variable(var.raw, strength.to_cassowary())
            .map_err(|e| SolverError::Contradiction(format!("Cannot add edit variable: {:?}", e)))
    }

    /// Suggests a new value for an edit variable.
    pub fn suggest_value(
        &mut self,
        var: ConstraintVariable,
        value: f64,
    ) -> Result<(), SolverError> {
        self.solver
            .suggest_value(var.raw, value)
            .map_err(|e| SolverError::Contradiction(format!("Cannot suggest value: {:?}", e)))
    }

    /// Evaluates and retrieves the solved value of a variable.
    pub fn get_value(&self, var: ConstraintVariable) -> f32 {
        let val = self.solver.get_value(var.raw);
        if val.is_finite() {
            // Quantize to 4 decimal places (0.0001 pt ~ 35 nm) to prevent floating-point
            // jitter across variable orderings and compilation targets from polluting downstream layout.
            ((val * 10_000.0).round() / 10_000.0) as f32
        } else {
            0.0
        }
    }

    /// Evaluates and retrieves a solved 2D rectangle.
    pub fn get_rect(&self, rect: RectVariables) -> SolvedRect {
        SolvedRect::new(
            self.get_value(rect.x),
            self.get_value(rect.y),
            self.get_value(rect.width),
            self.get_value(rect.height),
        )
    }
}

impl Default for GeometricConstraintSolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_satisfiable_linear_system() {
        let mut solver = GeometricConstraintSolver::new();
        let x = ConstraintVariable::new();
        let y = ConstraintVariable::new();

        // x + y == 100
        solver
            .add_eq(
                x.to_expr().plus(&y.to_expr()),
                LinearExpression::constant(100.0),
                ConstraintStrength::Required,
            )
            .unwrap();

        // x == y
        solver
            .add_eq(x.to_expr(), y.to_expr(), ConstraintStrength::Required)
            .unwrap();

        assert!((solver.get_value(x) - 50.0).abs() < 1e-4);
        assert!((solver.get_value(y) - 50.0).abs() < 1e-4);
    }

    #[test]
    fn test_inequality_bounds_and_suggestion() {
        let mut solver = GeometricConstraintSolver::new();
        let x = ConstraintVariable::new();

        // 10 <= x <= 20
        solver
            .add_ge(
                x.to_expr(),
                LinearExpression::constant(10.0),
                ConstraintStrength::Required,
            )
            .unwrap();
        solver
            .add_le(
                x.to_expr(),
                LinearExpression::constant(20.0),
                ConstraintStrength::Required,
            )
            .unwrap();

        solver
            .add_edit_variable(x, ConstraintStrength::Strong)
            .unwrap();

        // Suggest below minimum
        solver.suggest_value(x, 5.0).unwrap();
        assert!((solver.get_value(x) - 10.0).abs() < 1e-4);

        // Suggest above maximum
        solver.suggest_value(x, 50.0).unwrap();
        assert!((solver.get_value(x) - 20.0).abs() < 1e-4);

        // Suggest inside bounds
        solver.suggest_value(x, 14.5).unwrap();
        assert!((solver.get_value(x) - 14.5).abs() < 1e-4);
    }

    #[test]
    fn test_strength_prioritization() {
        let mut solver = GeometricConstraintSolver::new();
        let x = ConstraintVariable::new();

        // Weak wants x = 100
        solver
            .add_eq(
                x.to_expr(),
                LinearExpression::constant(100.0),
                ConstraintStrength::Weak,
            )
            .unwrap();

        // Strong wants x = 250
        solver
            .add_eq(
                x.to_expr(),
                LinearExpression::constant(250.0),
                ConstraintStrength::Strong,
            )
            .unwrap();

        // Strong must dominate Weak
        assert!((solver.get_value(x) - 250.0).abs() < 1e-4);
    }

    #[test]
    fn test_conflicting_required_constraints_produce_contradiction() {
        let mut solver = GeometricConstraintSolver::new();
        let x = ConstraintVariable::new();

        solver
            .add_eq(
                x.to_expr(),
                LinearExpression::constant(50.0),
                ConstraintStrength::Required,
            )
            .unwrap();

        // Contradictory required constraint: x == 80
        let result = solver.add_eq(
            x.to_expr(),
            LinearExpression::constant(80.0),
            ConstraintStrength::Required,
        );

        assert!(result.is_err());
        match result {
            Err(SolverError::Contradiction(_)) => {}
            other => panic!("Expected SolverError::Contradiction, got {:?}", other),
        }
    }

    #[test]
    fn test_conflicting_required_inequalities() {
        let mut solver = GeometricConstraintSolver::new();
        let x = ConstraintVariable::new();

        solver
            .add_ge(
                x.to_expr(),
                LinearExpression::constant(100.0),
                ConstraintStrength::Required,
            )
            .unwrap();

        // Contradiction: x <= 50 while x >= 100
        let result = solver.add_le(
            x.to_expr(),
            LinearExpression::constant(50.0),
            ConstraintStrength::Required,
        );

        assert!(result.is_err());
        match result {
            Err(SolverError::Contradiction(_)) => {}
            other => panic!("Expected SolverError::Contradiction, got {:?}", other),
        }
    }

    #[test]
    fn test_rect_variables_and_get_rect() {
        let mut solver = GeometricConstraintSolver::new();
        let rect = RectVariables::new();

        solver
            .add_eq(
                rect.x.to_expr(),
                LinearExpression::constant(40.0),
                ConstraintStrength::Required,
            )
            .unwrap();
        solver
            .add_eq(
                rect.y.to_expr(),
                LinearExpression::constant(50.0),
                ConstraintStrength::Required,
            )
            .unwrap();
        solver
            .add_eq(
                rect.width.to_expr(),
                LinearExpression::constant(200.0),
                ConstraintStrength::Required,
            )
            .unwrap();
        solver
            .add_eq(
                rect.height.to_expr(),
                LinearExpression::constant(300.0),
                ConstraintStrength::Required,
            )
            .unwrap();

        let solved = solver.get_rect(rect);
        assert!((solved.x - 40.0).abs() < 1e-4);
        assert!((solved.y - 50.0).abs() < 1e-4);
        assert!((solved.width - 200.0).abs() < 1e-4);
        assert!((solved.height - 300.0).abs() < 1e-4);
        assert!((solved.right() - 240.0).abs() < 1e-4);
        assert!((solved.bottom() - 350.0).abs() < 1e-4);
    }
}
