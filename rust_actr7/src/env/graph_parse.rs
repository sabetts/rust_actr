//! Parse the canvas-item production-graph records used by the Lisp
//! Environment (and emitted by the Rust history processors) into the typed
//! graph the native viewer renders.

use std::collections::HashMap;

use crate::engine::{
    ProductionGraph, ProductionGraphEdge, ProductionGraphMode, ProductionGraphNode,
    ProductionGraphPage,
};
use crate::json::Json;

/// Convert the history processor's canvas records into a graph.
///
/// Each page is an array of `[kind, …]` items. The first record may be a
/// `["size", width, height]` marker, and a transfer may wrap the size/pages in
/// one outer array, so both shapes are normalized here.
pub(crate) fn from_canvas_items(
    records: &[Json],
    mode: ProductionGraphMode,
) -> Option<ProductionGraph> {
    let mut pages = Vec::new();
    collect_pages(records, &mut pages);
    if pages.is_empty() {
        return None;
    }

    let mut nodes: Vec<ProductionGraphNode> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut edges: Vec<ProductionGraphEdge> = Vec::new();
    let mut graph_pages = Vec::new();

    for (page, items) in pages.iter().enumerate() {
        let page = page as u32;
        let mut start_ms = None;
        let mut end_ms = None;
        for item in *items {
            let Json::Arr(fields) = item else {
                continue;
            };
            let Some(kind) = fields.first().and_then(Json::as_str) else {
                continue;
            };
            match kind {
                "label" => {
                    let name = string(fields.get(1));
                    if name.is_empty() {
                        continue;
                    }
                    let i = ensure_node(&mut nodes, &mut index, &name);
                    let text = string(fields.get(9));
                    if !text.is_empty() {
                        nodes[i].text = text;
                    }
                    let color = string(fields.get(8));
                    if color.eq_ignore_ascii_case("gray") {
                        // Leave used as-is; another page may have used it.
                    } else {
                        nodes[i].used = true;
                    }
                }
                "box" => {
                    let name = string(fields.get(1));
                    if name.is_empty() {
                        continue;
                    }
                    let i = ensure_node(&mut nodes, &mut index, &name);
                    let color = string(fields.get(6));
                    if color.eq_ignore_ascii_case("green") {
                        mark(&mut nodes[i].starts, page);
                        nodes[i].used = true;
                    } else if color.eq_ignore_ascii_case("red") {
                        mark(&mut nodes[i].ends, page);
                        nodes[i].used = true;
                    } else if fields.get(7).and_then(Json::as_f64).unwrap_or(0.0) != 0.0 {
                        nodes[i].used = true;
                    }
                }
                "link" => {
                    let from = string(fields.get(1));
                    let to = string(fields.get(2));
                    if from.is_empty() || to.is_empty() {
                        continue;
                    }
                    ensure_node(&mut nodes, &mut index, &from);
                    ensure_node(&mut nodes, &mut index, &to);
                    let color = string(fields.get(3));
                    let width = fields
                        .get(4)
                        .and_then(Json::as_f64)
                        .unwrap_or(1.0)
                        .max(1.0)
                        .round() as u32;
                    edges.push(ProductionGraphEdge {
                        from,
                        to,
                        count: width,
                        page,
                        dashed: color.eq_ignore_ascii_case("gray"),
                        group: None,
                    });
                }
                "utility" => {
                    let name = string(fields.get(1));
                    let value = fields.get(2).and_then(Json::as_f64);
                    if let (false, Some(value)) = (name.is_empty(), value) {
                        let i = ensure_node(&mut nodes, &mut index, &name);
                        nodes[i].used = true;
                        nodes[i].utilities.push((page, value));
                    }
                }
                "min_time" => {
                    start_ms = fields
                        .get(1)
                        .and_then(Json::as_f64)
                        .map(|v| v.round() as i64);
                }
                "max_time" => {
                    end_ms = fields
                        .get(1)
                        .and_then(Json::as_f64)
                        .map(|v| v.round() as i64);
                }
                _ => {}
            }
        }
        graph_pages.push(ProductionGraphPage { start_ms, end_ms });
    }

    if nodes.is_empty() {
        return None;
    }
    let mut utility_min = None;
    let mut utility_max = None;
    for node in &nodes {
        for (_, value) in &node.utilities {
            utility_min = Some(utility_min.map_or(*value, |m: f64| m.min(*value)));
            utility_max = Some(utility_max.map_or(*value, |m: f64| m.max(*value)));
        }
    }
    if let (Some(min), Some(max)) = (utility_min, utility_max) {
        if min == max {
            utility_min = Some(min - 1.0);
            utility_max = Some(max + 1.0);
        }
    }

    Some(ProductionGraph {
        mode,
        nodes,
        edges,
        pages: graph_pages,
        utility_min,
        utility_max,
    })
}

fn collect_pages<'a>(records: &'a [Json], pages: &mut Vec<&'a [Json]>) {
    for record in records {
        let Json::Arr(items) = record else {
            continue;
        };
        if is_size(items) {
            continue;
        }
        if is_page(items) {
            pages.push(items);
        } else if items.iter().all(|item| matches!(item, Json::Arr(_))) {
            collect_pages(items, pages);
        }
    }
}

fn is_size(items: &[Json]) -> bool {
    items.first().and_then(Json::as_str) == Some("size")
}

fn is_page(items: &[Json]) -> bool {
    let Some(kind) = items.first().and_then(|item| match item {
        Json::Arr(fields) => fields.first().and_then(Json::as_str),
        _ => None,
    }) else {
        return false;
    };
    matches!(
        kind,
        "label" | "box" | "link" | "min_time" | "max_time" | "utility"
    )
}

fn ensure_node(
    nodes: &mut Vec<ProductionGraphNode>,
    index: &mut HashMap<String, usize>,
    name: &str,
) -> usize {
    if let Some(i) = index.get(name) {
        return *i;
    }
    let i = nodes.len();
    nodes.push(ProductionGraphNode {
        name: name.to_string(),
        ..Default::default()
    });
    index.insert(name.to_string(), i);
    i
}

fn mark(pages: &mut Vec<u32>, page: u32) {
    if !pages.contains(&page) {
        pages.push(page);
    }
}

fn string(value: Option<&Json>) -> String {
    value.and_then(Json::as_str).unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(value: &str) -> Json {
        Json::Str(value.into())
    }

    fn page() -> Json {
        Json::Arr(vec![
            Json::Arr(vec![
                record("label"),
                record("P1"),
                Json::Int(20),
                Json::Int(20),
                Json::Int(0),
                Json::Int(0),
                Json::Int(40),
                Json::Int(40),
                record("black"),
                record("(p1)"),
            ]),
            Json::Arr(vec![
                record("label"),
                record("P2"),
                Json::Int(80),
                Json::Int(20),
                Json::Int(60),
                Json::Int(0),
                Json::Int(100),
                Json::Int(40),
                record("gray"),
                record("(p2)"),
            ]),
            Json::Arr(vec![
                record("label"),
                record("P3"),
                Json::Int(140),
                Json::Int(20),
                Json::Int(120),
                Json::Int(0),
                Json::Int(160),
                Json::Int(40),
                record("black"),
                record("(p3)"),
            ]),
            Json::Arr(vec![
                record("box"),
                record("P1"),
                Json::Int(0),
                Json::Int(0),
                Json::Int(1),
                Json::Int(1),
                record("green"),
                Json::Int(1),
            ]),
            Json::Arr(vec![
                record("box"),
                record("P3"),
                Json::Int(0),
                Json::Int(0),
                Json::Int(1),
                Json::Int(1),
                record("red"),
                Json::Int(1),
            ]),
            Json::Arr(vec![
                record("link"),
                record("P1"),
                record("P2"),
                record("black"),
                Json::Int(2),
                Json::Int(1),
            ]),
            Json::Arr(vec![
                record("link"),
                record("P1"),
                record("P3"),
                record("gray"),
                Json::Int(1),
                Json::Int(1),
            ]),
            Json::Arr(vec![record("utility"), record("P1"), Json::Float(0.75)]),
            Json::Arr(vec![record("min_time"), Json::Int(100)]),
            Json::Arr(vec![record("max_time"), Json::Int(350)]),
        ])
    }

    #[test]
    fn parses_canvas_labels_boxes_links_and_pages() {
        let records = vec![
            Json::Arr(vec![record("size"), Json::Int(200), Json::Int(100)]),
            page(),
        ];
        let graph = from_canvas_items(&records, ProductionGraphMode::Frequencies).unwrap();
        assert_eq!(graph.mode, ProductionGraphMode::Frequencies);
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.nodes[0].name, "P1");
        assert_eq!(graph.nodes[0].text, "(p1)");
        assert!(graph.nodes[0].used);
        assert!(!graph.nodes[1].used);
        assert_eq!(graph.nodes[0].starts, vec![0]);
        assert_eq!(graph.nodes[2].ends, vec![0]);
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.edges[0].count, 2);
        assert!(!graph.edges[0].dashed);
        assert!(graph.edges[1].dashed);
        assert_eq!(graph.pages[0].start_ms, Some(100));
        assert_eq!(graph.pages[0].end_ms, Some(350));
        assert_eq!(graph.nodes[0].utilities, vec![(0, 0.75)]);
    }

    #[test]
    fn flattens_a_wrapped_size_and_page_transfer() {
        let records = vec![Json::Arr(vec![
            Json::Arr(vec![record("size"), Json::Int(200), Json::Int(100)]),
            page(),
        ])];
        let graph = from_canvas_items(&records, ProductionGraphMode::All).unwrap();
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.pages.len(), 1);
    }

    #[test]
    fn rejects_empty_or_unstructured_records() {
        assert!(from_canvas_items(&[], ProductionGraphMode::All).is_none());
        assert!(from_canvas_items(
            &[Json::Arr(vec![record("size"), Json::Int(1), Json::Int(1)])],
            ProductionGraphMode::All
        )
        .is_none());
    }
}
