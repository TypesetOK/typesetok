use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tok_core::anchor::TextAnchor;
use tok_core::id::NodeId;
use tok_core::transaction::CompoundTransaction;

/// High-speed binary IPC command sent from UI frontend to Rust Core.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum IpcCommand {
    InsertText {
        anchor: TextAnchor,
        text: String,
    },
    DeleteRange {
        start: TextAnchor,
        end: TextAnchor,
    },
    ApplyTransaction {
        transaction: CompoundTransaction,
    },
    QueryGeometry {
        page_index: u32,
    },
    HitTest {
        page_index: u32,
        x_pt: f32,
        y_pt: f32,
    },
    RequestRender {
        format: String, // "pdf" or "html"
        output_path: String,
    },
    ConfigureDropCap {
        paragraph_id: NodeId,
        config: DropCapIpcConfig,
    },
    AddExclusionZone {
        frame_id: String,
        zone: ExclusionZoneIpcPayload,
    },
    LinkMultiFlowAnchor {
        primary_node_id: NodeId,
        commentary_node_id: NodeId,
        anchor_id: String,
        commentary_flow_id: String,
    },
    BalanceColumnHeights {
        page_index: u32,
        config: Option<HeightBalanceIpcConfig>,
    },
    ConfigureHeadingNumbering {
        config: tok_core::outline::OutlineNumberingConfig,
    },
    ReorderHeadingNode {
        node_id: NodeId,
        new_parent_id: Option<NodeId>,
        target_index: usize,
    },
    InsertHeadingNode {
        parent_id: Option<NodeId>,
        target_index: usize,
        node: tok_core::outline::DocumentHierarchyNode,
    },
    DeleteHeadingNode {
        node_id: NodeId,
    },
    RecalculateHeadingNumbers,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropCapIpcConfig {
    pub lines: usize,
    pub characters_count: usize,
    pub distance_pt: f32,
    pub is_bold: bool,
    pub font_family: Option<String>,
    pub scale_factor: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExclusionZoneIpcPayload {
    pub id: String,
    pub x_pt: f32,
    pub y_pt: f32,
    pub width_pt: f32,
    pub height_pt: f32,
    pub standoff_pt: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeightBalanceIpcConfig {
    pub max_line_expansion_pt: f32,
    pub max_paragraph_expansion_pt: f32,
    pub min_lines_for_balance: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnBalanceIpcReport {
    pub flow_id: String,
    pub original_bottom_y_pt: f32,
    pub balanced_bottom_y_pt: f32,
    pub spring_injected_pt: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlyphGeometry {
    pub cluster: u32,
    pub x_pt: f32,
    pub y_pt: f32,
    pub width_pt: f32,
    pub height_pt: f32,
    pub character: Option<char>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineGeometry {
    pub line_index: u32,
    pub baseline_y_pt: f32,
    pub height_pt: f32,
    #[serde(default)]
    pub x_offset_pt: f32,
    #[serde(default)]
    pub is_drop_cap: bool,
    pub text: String,
    pub glyphs: Vec<GlyphGeometry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageGeometryPayload {
    pub page_index: u32,
    pub gematria_number: String,
    pub width_pt: f32,
    pub height_pt: f32,
    pub lines: Vec<LineGeometry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HitTestResult {
    pub node_id: NodeId,
    pub char_offset: usize,
    pub visual_x_pt: f32,
    pub visual_y_pt: f32,
    pub line_index: usize,
}

/// High-speed binary IPC event sent from Rust Core to UI frontend.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum IpcEvent {
    TransactionApplied {
        revision: u64,
        affected_pages: Vec<u32>,
    },
    GeometryUpdate {
        page: PageGeometryPayload,
    },
    HitTestResponse(HitTestResult),
    RenderCompleted {
        output_path: String,
        bytes_count: usize,
    },
    AnchorCascadeTriggered {
        displaced_primary_node: NodeId,
        cascaded_commentaries: Vec<(String, NodeId, u32)>,
    },
    ColumnsBalanced {
        page_index: u32,
        balanced_bottom_baseline_pt: f32,
        reports: Vec<ColumnBalanceIpcReport>,
    },
    HeadingNumberingUpdated {
        outline: tok_core::outline::DocumentOutline,
        affected_paragraphs: Vec<(NodeId, String)>,
    },
    Error {
        code: u32,
        message: String,
    },
}

use std::fmt;

#[derive(Debug)]
pub enum IpcError {
    Serialization(serde_json::Error),
    MessageTooLarge,
    PayloadTooLarge,
    InvalidLength,
}

impl fmt::Display for IpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serialization(e) => write!(f, "Serialization error: {}", e),
            Self::MessageTooLarge => write!(f, "Message exceeds MAX_MESSAGE_SIZE"),
            Self::PayloadTooLarge => write!(f, "Payload length exceeds u32::MAX"),
            Self::InvalidLength => write!(f, "Invalid message length or incomplete payload"),
        }
    }
}

impl std::error::Error for IpcError {}

impl From<serde_json::Error> for IpcError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serialization(e)
    }
}

pub const MAX_MESSAGE_SIZE: u32 = 64 * 1024 * 1024;

/// Length of the little-endian `u32` frame header.
pub const FRAME_HEADER_LEN: usize = 4;

/// Binary message framing (4-byte little-endian length prefix + JSON payload).
pub struct MessageFramer;

impl MessageFramer {
    fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, IpcError> {
        let payload = serde_json::to_vec(value)?;
        let len = u32::try_from(payload.len()).map_err(|_| IpcError::PayloadTooLarge)?;
        if len > MAX_MESSAGE_SIZE {
            return Err(IpcError::MessageTooLarge);
        }
        let mut framed = Vec::with_capacity(FRAME_HEADER_LEN + payload.len());
        framed.extend_from_slice(&len.to_le_bytes());
        framed.extend_from_slice(&payload);
        Ok(framed)
    }

    /// Splits the first complete frame off a byte stream.
    ///
    /// Returns `Ok(None)` while the frame is still incomplete (read more
    /// bytes), `Ok(Some((payload, consumed)))` once it is complete, and an
    /// error as soon as the header announces a frame above
    /// [`MAX_MESSAGE_SIZE`], before any of it has to be buffered.
    pub fn split_frame(bytes: &[u8]) -> Result<Option<(&[u8], usize)>, IpcError> {
        let Some(header) = bytes.first_chunk::<FRAME_HEADER_LEN>() else {
            return Ok(None);
        };
        let len = u32::from_le_bytes(*header);
        if len > MAX_MESSAGE_SIZE {
            return Err(IpcError::MessageTooLarge);
        }
        let end = FRAME_HEADER_LEN + len as usize;
        Ok(bytes
            .get(FRAME_HEADER_LEN..end)
            .map(|payload| (payload, end)))
    }

    /// Decodes the frame at the start of `bytes`. Bytes after the frame are
    /// ignored; use [`MessageFramer::split_frame`] to consume a stream.
    fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, IpcError> {
        match Self::split_frame(bytes)? {
            Some((payload, _)) => Ok(serde_json::from_slice(payload)?),
            None => Err(IpcError::InvalidLength),
        }
    }

    pub fn encode_command(cmd: &IpcCommand) -> Result<Vec<u8>, IpcError> {
        Self::encode(cmd)
    }

    pub fn decode_command(bytes: &[u8]) -> Result<IpcCommand, IpcError> {
        Self::decode(bytes)
    }

    pub fn encode_event(evt: &IpcEvent) -> Result<Vec<u8>, IpcError> {
        Self::encode(evt)
    }

    pub fn decode_event(bytes: &[u8]) -> Result<IpcEvent, IpcError> {
        Self::decode(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(len: u32, payload: &[u8]) -> Vec<u8> {
        let mut v = len.to_le_bytes().to_vec();
        v.extend_from_slice(payload);
        v
    }

    #[test]
    fn oversized_header_is_rejected_before_buffering() {
        let bytes = frame(MAX_MESSAGE_SIZE + 1, b"");
        assert!(matches!(
            MessageFramer::decode_command(&bytes),
            Err(IpcError::MessageTooLarge)
        ));
        assert!(matches!(
            MessageFramer::split_frame(&bytes),
            Err(IpcError::MessageTooLarge)
        ));
        assert!(matches!(
            MessageFramer::decode_event(&frame(u32::MAX, b"{}")),
            Err(IpcError::MessageTooLarge)
        ));
    }

    #[test]
    fn short_and_truncated_frames() {
        for bytes in [&[][..], &[1, 0][..], &[5, 0, 0, 0, b'{'][..]] {
            assert!(matches!(
                MessageFramer::decode_event(bytes),
                Err(IpcError::InvalidLength)
            ));
            assert!(MessageFramer::split_frame(bytes).unwrap().is_none());
        }
        // Zero-length payload is a complete (but invalid JSON) frame.
        assert!(matches!(
            MessageFramer::decode_event(&frame(0, b"")),
            Err(IpcError::Serialization(_))
        ));
    }

    #[test]
    fn stream_of_frames_is_split_exactly() {
        let a = MessageFramer::encode_event(&IpcEvent::Error {
            code: 1,
            message: "א".to_string(),
        })
        .unwrap();
        let b =
            MessageFramer::encode_command(&IpcCommand::QueryGeometry { page_index: 7 }).unwrap();
        let mut stream = a.clone();
        stream.extend_from_slice(&b);

        let (payload, used) = MessageFramer::split_frame(&stream).unwrap().unwrap();
        assert_eq!(used, a.len());
        assert_eq!(payload, &a[FRAME_HEADER_LEN..]);
        let rest = &stream[used..];
        let (_, used_b) = MessageFramer::split_frame(rest).unwrap().unwrap();
        assert_eq!(used_b, b.len());
        assert_eq!(
            MessageFramer::decode_command(rest).unwrap(),
            IpcCommand::QueryGeometry { page_index: 7 }
        );
    }

    #[test]
    fn wire_format_is_unchanged() {
        let bytes =
            MessageFramer::encode_command(&IpcCommand::QueryGeometry { page_index: 3 }).unwrap();
        let json = br#"{"type":"QueryGeometry","payload":{"page_index":3}}"#;
        assert_eq!(bytes, frame(json.len() as u32, json));
    }
}
