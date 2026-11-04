//! Typed history views for the native Environment.
//!
//! The RPC layer returns each history as JSON records. These types parse the
//! records into the same information the Tcl history dialogs show, so the UI
//! can render the split panes without re-reading ad-hoc JSON every frame.

use crate::json::Json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrievalChunk {
    pub name: String,
    pub text: String,
    pub params: String,
    pub trace: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrievalEntry {
    pub time_ms: i64,
    pub request: String,
    pub chunks: Vec<RetrievalChunk>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlendingChunk {
    pub name: String,
    pub activation: String,
    pub probability: String,
    pub printed: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlendingEntry {
    pub time_ms: i64,
    pub request: String,
    pub result: String,
    pub chunks: Vec<BlendingChunk>,
    pub trace: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferAction {
    pub buffer: String,
    pub action: String,
    pub chunk: String,
    pub status_after: String,
    pub previous_chunk: String,
    pub previous_status: String,
    pub current_chunk: String,
    pub current_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferEntry {
    pub time_ms: i64,
    pub actions: Vec<BufferAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerceptualEntry {
    pub time_ms: i64,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryView {
    Retrieval(Vec<RetrievalEntry>),
    Blending(Vec<BlendingEntry>),
    Buffer(Vec<BufferEntry>),
    Perceptual(Vec<PerceptualEntry>),
}

pub fn parse_retrieval(records: &[Json]) -> Vec<RetrievalEntry> {
    records
        .iter()
        .filter_map(|record| {
            let fields = array(record)?;
            let chunks = array(fields.get(2)?)?
                .iter()
                .filter_map(|chunk: &Json| {
                    let c = array(chunk)?;
                    Some(RetrievalChunk {
                        name: text(c.first()),
                        text: text(c.get(1)),
                        params: text(c.get(2)),
                        trace: text(c.get(3)),
                    })
                })
                .collect();
            Some(RetrievalEntry {
                time_ms: time(fields.first()),
                request: text(fields.get(1)),
                chunks,
            })
        })
        .collect()
}

pub fn parse_blending(records: &[Json]) -> Vec<BlendingEntry> {
    records
        .iter()
        .filter_map(|record| {
            let fields = array(record)?;
            let chunks = array(fields.get(3)?)?
                .iter()
                .filter_map(|chunk: &Json| {
                    let c = array(chunk)?;
                    Some(BlendingChunk {
                        name: text(c.first()),
                        activation: text(c.get(1)),
                        probability: text(c.get(2)),
                        printed: text(c.get(3)),
                    })
                })
                .collect();
            Some(BlendingEntry {
                time_ms: time(fields.first()),
                request: text(fields.get(1)),
                result: text(fields.get(2)),
                chunks,
                trace: text(fields.get(4)),
            })
        })
        .collect()
}

pub fn parse_buffer(records: &[Json]) -> Vec<BufferEntry> {
    records
        .iter()
        .filter_map(|record| {
            let fields = array(record)?;
            let actions = array(fields.get(1)?)?
                .iter()
                .filter_map(|action: &Json| {
                    let a = array(action)?;
                    Some(BufferAction {
                        buffer: text(a.first()),
                        action: text(a.get(1)),
                        chunk: text(a.get(2)),
                        status_after: text(a.get(3)),
                        previous_chunk: text(a.get(4)),
                        previous_status: text(a.get(5)),
                        current_chunk: text(a.get(6)),
                        current_status: text(a.get(7)),
                    })
                })
                .collect();
            Some(BufferEntry {
                time_ms: time(fields.first()),
                actions,
            })
        })
        .collect()
}

pub fn parse_perceptual(records: &[Json]) -> Vec<PerceptualEntry> {
    records
        .iter()
        .filter_map(|record| {
            let fields = array(record)?;
            let mut text = String::new();
            for value in fields.iter().skip(1) {
                text.push_str(&text_of(Some(value)));
            }
            Some(PerceptualEntry {
                time_ms: time(fields.first()),
                text,
            })
        })
        .collect()
}

pub fn format_time(time_ms: i64) -> String {
    format!("{:.3}", time_ms as f64 / 1000.0)
}

fn array(value: &Json) -> Option<&[Json]> {
    match value {
        Json::Arr(items) => Some(items),
        _ => None,
    }
}

fn time(value: Option<&Json>) -> i64 {
    value.and_then(|x| x.as_f64()).unwrap_or(0.0).round() as i64
}

fn text(value: Option<&Json>) -> String {
    text_of(value)
}

fn text_of(value: Option<&Json>) -> String {
    value
        .map(crate::rpc::session::json_to_text)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_retrieval_records() {
        let records = vec![Json::Arr(vec![
            Json::Int(250),
            Json::Str("+retrieval>".into()),
            Json::Arr(vec![Json::Arr(vec![
                Json::Str("CHUNK0".into()),
                Json::Str("chunk text".into()),
                Json::Str("params".into()),
                Json::Str("activation".into()),
            ])]),
        ])];
        let view = parse_retrieval(&records);
        assert_eq!(view.len(), 1);
        assert_eq!(view[0].time_ms, 250);
        assert_eq!(view[0].chunks[0].name, "CHUNK0");
        assert_eq!(view[0].chunks[0].trace, "activation");
    }

    #[test]
    fn parses_blending_buffer_and_perceptual() {
        let blending = parse_blending(&[Json::Arr(vec![
            Json::Int(300),
            Json::Str("+blending>".into()),
            Json::Str("result".into()),
            Json::Arr(vec![Json::Arr(vec![
                Json::Str("CHUNK0".into()),
                Json::Float(1.5),
                Json::Float(0.5),
                Json::Str("printed".into()),
            ])]),
            Json::Str("trace".into()),
        ])]);
        assert_eq!(blending[0].chunks[0].activation, "1.5");
        assert_eq!(blending[0].trace, "trace");

        let buffer = parse_buffer(&[Json::Arr(vec![
            Json::Int(400),
            Json::Arr(vec![Json::Arr(vec![
                Json::Str("goal".into()),
                Json::Str("set-buffer-chunk".into()),
                Json::Str("chunk".into()),
                Json::Str("free".into()),
                Json::Str("old".into()),
                Json::Str("free".into()),
                Json::Str("new".into()),
                Json::Str("busy".into()),
            ])]),
        ])]);
        assert_eq!(buffer[0].actions[0].action, "set-buffer-chunk");
        assert_eq!(buffer[0].actions[0].current_status, "busy");

        let perceptual = parse_perceptual(&[Json::Arr(vec![
            Json::Int(500),
            Json::Str("TEXT0".into()),
            Json::Str("TEXT1".into()),
        ])]);
        assert_eq!(perceptual[0].text, "TEXT0TEXT1");
    }
}
