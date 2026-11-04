//! PostScript/DOT writers for the native Environment export actions.

use std::collections::HashMap;

use crate::json::Json;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GridCell {
    Blank,
    Selected(String),
    Matched(String),
    Mismatched(String),
}

#[derive(Debug, Clone, Default)]
pub struct GridData {
    pub times: Vec<String>,
    pub productions: Vec<String>,
    pub production_text: Vec<String>,
    pub why_not: Vec<String>,
    /// `rows[production][decision]`.
    pub rows: Vec<Vec<GridCell>>,
}

impl GridData {
    pub fn parse(records: &[Json]) -> GridData {
        let mut data = GridData::default();
        let mut decisions = Vec::new();
        for record in records {
            let Json::Arr(fields) = record else {
                continue;
            };
            match fields.get(1).and_then(|x| x.as_str()) {
                Some("info") => {
                    if let Some(Json::Arr(why)) = fields.get(2) {
                        data.why_not = why
                            .iter()
                            .filter_map(|x| x.as_str().map(str::to_string))
                            .collect();
                    }
                    if let Some(Json::Arr(productions)) = fields.get(3) {
                        for production in productions {
                            if let Json::Arr(p) = production {
                                if let Some(name) = p.first().and_then(|x| x.as_str()) {
                                    data.productions.push(name.to_string());
                                }
                                data.production_text.push(json_text(p.get(1)));
                            }
                        }
                    }
                }
                Some("conflict-resolution") => {
                    let time = fields.first().and_then(|x| x.as_f64()).unwrap_or(0.0) / 1000.0;
                    data.times.push(format!("{time:.3}"));
                    decisions.push(fields.get(3).cloned().unwrap_or(Json::Null));
                }
                _ => {}
            }
        }
        data.rows = vec![Vec::new(); data.productions.len()];
        for decision in decisions {
            let Json::Arr(cells) = decision else {
                continue;
            };
            for (i, row) in data.rows.iter_mut().enumerate() {
                let cell = cells.get(i);
                row.push(match cell {
                    Some(Json::Arr(c)) => match c.first().and_then(|x| x.as_f64()) {
                        Some(0.0) => GridCell::Selected(json_text(c.get(1))),
                        Some(1.0) => GridCell::Matched(json_text(c.get(1))),
                        Some(2.0) => GridCell::Mismatched(json_text(c.get(1))),
                        _ => GridCell::Blank,
                    },
                    _ => GridCell::Blank,
                });
            }
        }
        data
    }

    pub fn visible_columns(&self, hide_empty: bool) -> Vec<usize> {
        if !hide_empty {
            return (0..self.times.len()).collect();
        }
        (0..self.times.len())
            .filter(|column| {
                self.rows
                    .iter()
                    .any(|row| matches!(row.get(*column), Some(GridCell::Selected(_))))
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct GraphNodeView {
    pub name: String,
    pub text: String,
    pub used: bool,
    pub start: bool,
    pub end: bool,
    pub utility: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct GraphEdgeView {
    pub from: String,
    pub to: String,
    pub count: u32,
    pub dashed: bool,
    pub group: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct GraphLayout {
    pub nodes: Vec<GraphNodeView>,
    pub edges: Vec<GraphEdgeView>,
    pub positions: HashMap<String, (f32, f32)>,
    pub width: f32,
    pub height: f32,
    pub page: u32,
    pub pages: u32,
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
    pub utility_min: Option<f64>,
    pub utility_max: Option<f64>,
}

pub const GRAPH_BOX_W: f32 = 130.0;
pub const GRAPH_BOX_H: f32 = 30.0;

pub fn layout_graph_page(
    data: &crate::engine::ProductionGraph,
    page: u32,
    hide_unused: bool,
) -> GraphLayout {
    let pages = data.pages.len().max(1) as u32;
    let page = page.min(pages - 1);
    let mut nodes: Vec<GraphNodeView> = data
        .nodes
        .iter()
        .map(|node| GraphNodeView {
            name: node.name.clone(),
            text: node.text.clone(),
            used: node.used,
            start: node.starts.contains(&page),
            end: node.ends.contains(&page),
            utility: node
                .utilities
                .iter()
                .find(|(p, _)| *p == page)
                .map(|(_, u)| *u),
        })
        .collect();
    let mut edges: Vec<GraphEdgeView> = data
        .edges
        .iter()
        .filter(|edge| edge.page == page)
        .map(|edge| GraphEdgeView {
            from: edge.from.clone(),
            to: edge.to.clone(),
            count: edge.count,
            dashed: edge.dashed,
            group: edge.group,
        })
        .collect();
    if hide_unused {
        nodes.retain(|node| node.used);
        edges.retain(|edge| {
            nodes.iter().any(|n| n.name == edge.from) && nodes.iter().any(|n| n.name == edge.to)
        });
    }
    let mut layer: HashMap<&str, usize> = nodes.iter().map(|n| (n.name.as_str(), 0usize)).collect();
    for _ in 0..nodes.len().min(20) {
        let mut changed = false;
        for edge in edges.iter().filter(|edge| !edge.dashed) {
            let la = layer.get(edge.from.as_str()).copied().unwrap_or(0);
            let lb = layer.get(edge.to.as_str()).copied().unwrap_or(0);
            if lb <= la {
                layer.insert(edge.to.as_str(), (la + 1).min(nodes.len()));
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let max_layer = *layer.values().max().unwrap_or(&0);
    let mut by_layer: Vec<Vec<&GraphNodeView>> = vec![Vec::new(); max_layer + 1];
    for node in &nodes {
        by_layer[layer.get(node.name.as_str()).copied().unwrap_or(0)].push(node);
    }
    let h_gap = 20.0;
    let v_gap = 70.0;
    let width =
        by_layer.iter().map(|l| l.len()).max().unwrap_or(1) as f32 * (GRAPH_BOX_W + h_gap) + 40.0;
    let height = (max_layer + 1) as f32 * (GRAPH_BOX_H + v_gap) + 40.0;
    let mut positions = HashMap::new();
    for (li, layer_nodes) in by_layer.iter().enumerate() {
        for (xi, node) in layer_nodes.iter().enumerate() {
            positions.insert(
                node.name.clone(),
                (
                    20.0 + xi as f32 * (GRAPH_BOX_W + h_gap),
                    20.0 + li as f32 * (GRAPH_BOX_H + v_gap),
                ),
            );
        }
    }
    GraphLayout {
        nodes,
        edges,
        positions,
        width,
        height,
        page,
        pages,
        start_ms: data.pages.get(page as usize).and_then(|p| p.start_ms),
        end_ms: data.pages.get(page as usize).and_then(|p| p.end_ms),
        utility_min: data.utility_min,
        utility_max: data.utility_max,
    }
}

pub fn grid_eps(
    data: &GridData,
    hide_empty: bool,
    selected: &str,
    matched: &str,
    mismatched: &str,
) -> String {
    let columns = data.visible_columns(hide_empty);
    grid_page(data, &columns, selected, matched, mismatched, 1, true)
}

pub fn grid_ps(
    data: &GridData,
    hide_empty: bool,
    selected: &str,
    matched: &str,
    mismatched: &str,
) -> String {
    let columns = data.visible_columns(hide_empty);
    let per_page = 12;
    let pages = if columns.is_empty() {
        1
    } else {
        (columns.len() + per_page - 1) / per_page
    };
    let mut out = String::from("%!PS-Adobe-3.0\n%%Pages: ");
    out.push_str(&pages.to_string());
    out.push_str("\n%%EndComments\n");
    for page in 0..pages {
        let start = page * per_page;
        let end = (start + per_page).min(columns.len());
        out.push_str(&grid_page(
            data,
            &columns[start..end],
            selected,
            matched,
            mismatched,
            page + 1,
            false,
        ));
        if page + 1 < pages {
            out.push_str("\nshowpage\n");
        }
    }
    out.push_str("\n%%EOF\n");
    out
}

fn grid_page(
    data: &GridData,
    columns: &[usize],
    selected: &str,
    matched: &str,
    mismatched: &str,
    page: usize,
    eps: bool,
) -> String {
    let cell_w = 74.0;
    let row_h = 18.0;
    let label_w = 150.0;
    let width = label_w + columns.len() as f32 * cell_w + 20.0;
    let height = 40.0 + data.productions.len() as f32 * row_h + 20.0;
    let mut out = String::new();
    if eps {
        out.push_str(&format!(
            "%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 {} {}\n",
            width.ceil(),
            height.ceil()
        ));
    } else {
        out.push_str(&format!(
            "%%Page: {page} {page}\n%%BoundingBox: 0 0 {} {}\n",
            width.ceil(),
            height.ceil()
        ));
    }
    out.push_str("/Helvetica findfont 8 scalefont setfont\n");
    out.push_str(&format!("10 {} moveto (Production) show\n", height - 20.0));
    for (i, column) in columns.iter().enumerate() {
        let x = label_w + 10.0 + i as f32 * cell_w;
        out.push_str(&format!(
            "{:.1} {} moveto ({}) show\n",
            x,
            height - 20.0,
            ps_escape(data.times.get(*column).map(String::as_str).unwrap_or(""))
        ));
    }
    for (r, production) in data.productions.iter().enumerate() {
        let y = height - 40.0 - r as f32 * row_h;
        out.push_str(&format!(
            "10 {y:.1} moveto ({}) show\n",
            ps_escape(production)
        ));
        for (i, column) in columns.iter().enumerate() {
            let x = label_w + 10.0 + i as f32 * cell_w;
            let cell = data
                .rows
                .get(r)
                .and_then(|row| row.get(*column))
                .unwrap_or(&GridCell::Blank);
            let (color, label) = match cell {
                GridCell::Selected(label) => (selected, label.as_str()),
                GridCell::Matched(label) => (matched, label.as_str()),
                GridCell::Mismatched(label) => (mismatched, label.as_str()),
                GridCell::Blank => ("", ""),
            };
            if !color.is_empty() {
                out.push_str(&format!("{} setrgbcolor\n", ps_rgb(color)));
                out.push_str(&format!(
                    "newpath {x:.1} {y:.1} moveto {} 14 rlineto 0 {row_h} rlineto -{} 0 rlineto closepath fill\n",
                    cell_w - 2.0,
                    cell_w - 2.0
                ));
            }
            out.push_str("0 setgray\n");
            out.push_str(&format!(
                "{:.1} {:.1} moveto ({}) show\n",
                x + 3.0,
                y + 4.0,
                ps_escape(label)
            ));
        }
    }
    out
}

pub fn graph_dot(layout: &GraphLayout) -> String {
    let mut out = format!(
        "digraph production_history {{\n  subgraph cluster_page_{} {{\n    label=\"Page {} of {}\";\n",
        layout.page,
        layout.page + 1,
        layout.pages
    );
    for node in &layout.nodes {
        let mut attrs = Vec::new();
        if node.start {
            attrs.push("color=green".to_string());
        } else if node.end {
            attrs.push("color=red".to_string());
        } else if !node.used {
            attrs.push("color=gray".to_string());
        }
        if attrs.is_empty() {
            out.push_str(&format!("    \"{}\" ;\n", dot_escape(&node.name)));
        } else {
            out.push_str(&format!(
                "    \"{}\" [{}] ;\n",
                dot_escape(&node.name),
                attrs.join(" ")
            ));
        }
    }
    for edge in &layout.edges {
        let mut attrs = Vec::new();
        if edge.dashed {
            attrs.push("style=dashed".to_string());
            attrs.push("color=gray".to_string());
        }
        if edge.count > 1 {
            attrs.push(format!("label=\"{}\"", edge.count));
        }
        if attrs.is_empty() {
            out.push_str(&format!(
                "    \"{}\" -> \"{}\" ;\n",
                dot_escape(&edge.from),
                dot_escape(&edge.to)
            ));
        } else {
            out.push_str(&format!(
                "    \"{}\" -> \"{}\" [{}] ;\n",
                dot_escape(&edge.from),
                dot_escape(&edge.to),
                attrs.join(" ")
            ));
        }
    }
    out.push_str("  }\n}\n");
    out
}

fn ps_color(out: &mut String, color: (f32, f32, f32)) {
    out.push_str(&format!(
        "{:.3} {:.3} {:.3} setrgbcolor\n",
        color.0, color.1, color.2
    ));
}

fn group_color(group: Option<u32>) -> (f32, f32, f32) {
    const COLORS: [(f32, f32, f32); 8] = [
        (0.0, 0.0, 0.0),
        (0.80, 0.10, 0.10),
        (0.10, 0.30, 0.85),
        (0.10, 0.60, 0.20),
        (0.90, 0.45, 0.05),
        (0.55, 0.10, 0.65),
        (0.00, 0.55, 0.60),
        (0.55, 0.40, 0.05),
    ];
    COLORS[group.unwrap_or(0) as usize % COLORS.len()]
}

pub fn graph_eps(layout: &GraphLayout) -> String {
    let mut out = format!(
        "%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 {} {}\n",
        layout.width.ceil(),
        layout.height.ceil()
    );
    out.push_str("/Helvetica findfont 8 scalefont setfont\n");
    for edge in &layout.edges {
        let (Some(&(ax, ay)), Some(&(bx, by))) = (
            layout.positions.get(&edge.from),
            layout.positions.get(&edge.to),
        ) else {
            continue;
        };
        let x1 = ax + GRAPH_BOX_W / 2.0;
        let y1 = layout.height - (ay + GRAPH_BOX_H);
        let x2 = bx + GRAPH_BOX_W / 2.0;
        let y2 = layout.height - by;
        if edge.dashed {
            ps_color(&mut out, (0.5, 0.5, 0.5));
            out.push_str("[3 3] 0 setdash\n");
        } else if edge.group.is_some() {
            ps_color(&mut out, group_color(edge.group));
        } else {
            ps_color(&mut out, (0.0, 0.0, 0.0));
        }
        let width = 1.0 + (edge.count.saturating_sub(1) as f32).min(5.0);
        if edge.from == edge.to {
            out.push_str(&format!(
                "newpath {x1:.1} {y1:.1} moveto 20 18 rlineto 0 24 rlineto -40 0 rlineto 0 -24 rlineto stroke\n"
            ));
        } else {
            out.push_str(&format!(
                "{width:.1} setlinewidth newpath {x1:.1} {y1:.1} moveto {x2:.1} {y2:.1} lineto stroke\n"
            ));
        }
        out.push_str("[] 0 setdash 1 setlinewidth\n");
    }
    for node in &layout.nodes {
        let Some(&(x, y)) = layout.positions.get(&node.name) else {
            continue;
        };
        let py = layout.height - (y + GRAPH_BOX_H);
        out.push_str(&format!(
            "0.95 setgray newpath {x:.1} {py:.1} moveto {GRAPH_BOX_W:.1} 0 rlineto 0 {GRAPH_BOX_H:.1} rlineto -{GRAPH_BOX_W:.1} 0 rlineto closepath fill\n"
        ));
        let border = if node.start {
            (0.0, 0.55, 0.0)
        } else if node.end {
            (0.80, 0.05, 0.05)
        } else if !node.used {
            (0.55, 0.55, 0.55)
        } else {
            (0.0, 0.0, 0.0)
        };
        ps_color(&mut out, border);
        out.push_str(&format!(
            "2 setlinewidth newpath {x:.1} {py:.1} moveto {GRAPH_BOX_W:.1} 0 rlineto 0 {GRAPH_BOX_H:.1} rlineto -{GRAPH_BOX_W:.1} 0 rlineto closepath stroke\n"
        ));
        out.push_str("1 setlinewidth 0 setgray\n");
        out.push_str(&format!(
            "{:.1} {:.1} moveto ({}) show\n",
            x + 4.0,
            py + 10.0,
            ps_escape(&node.name)
        ));
        if let (Some(value), Some(min), Some(max)) =
            (node.utility, layout.utility_min, layout.utility_max)
        {
            let frac = if max == min {
                1.0
            } else {
                ((value - min) / (max - min)).clamp(0.0, 1.0) as f32
            };
            let bar_w = (GRAPH_BOX_W - 8.0) * frac;
            ps_color(&mut out, (0.10, 0.30, 0.85));
            out.push_str(&format!(
                "newpath {:.1} {:.1} moveto {bar_w:.1} 0 rlineto 0 4 rlineto -{bar_w:.1} 0 rlineto closepath fill\n",
                x + 4.0,
                py - 6.0
            ));
        }
    }
    out.push_str("0 setgray\n");
    let mut footer = format!("Page {} of {}", layout.page + 1, layout.pages);
    if let (Some(start), Some(end)) = (layout.start_ms, layout.end_ms) {
        footer.push_str(&format!(
            "   Time {:.3} - {:.3}",
            start as f64 / 1000.0,
            end as f64 / 1000.0
        ));
    }
    out.push_str(&format!("10 10 moveto ({}) show\n", ps_escape(&footer)));
    out.push_str("%%EOF\n");
    out
}

fn json_text(value: Option<&Json>) -> String {
    value
        .map(crate::rpc::session::json_to_text)
        .unwrap_or_default()
}

fn ps_rgb(hex: &str) -> String {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return "0 0 0".to_string();
    }
    let parse = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0) as f64 / 255.0;
    format!("{:.3} {:.3} {:.3}", parse(0), parse(2), parse(4))
}

fn ps_escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
        .replace(['\n', '\r'], " ")
}

fn dot_escape(text: &str) -> String {
    text.replace('"', "\\\"")
}

#[derive(Debug, Clone, Default)]
pub struct GraphicTraceLabel {
    pub name: String,
    pub pos: f32,
    pub index: usize,
}

#[derive(Debug, Clone, Default)]
pub struct GraphicTraceRect {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub index: usize,
    pub text: String,
    pub request: String,
    pub chunk: String,
    pub notes: String,
    pub time_start: f64,
    pub time_end: f64,
}

#[derive(Debug, Clone, Default)]
pub struct GraphicTraceData {
    pub max_time: f64,
    pub labels: Vec<GraphicTraceLabel>,
    pub rects: Vec<GraphicTraceRect>,
}

impl GraphicTraceData {
    pub fn parse(records: &[Json], vertical: bool) -> Self {
        let mut data = GraphicTraceData::default();
        for record in records {
            let Json::Arr(fields) = record else {
                continue;
            };
            match fields.first().and_then(|x| x.as_str()) {
                Some("size") => {
                    data.max_time = fields.get(1).and_then(|x| x.as_f64()).unwrap_or(0.0);
                }
                Some("label") => data.labels.push(GraphicTraceLabel {
                    name: json_text(fields.get(1)),
                    pos: fields.get(2).and_then(|x| x.as_f64()).unwrap_or(0.0) as f32,
                    index: fields.get(3).and_then(|x| x.as_f64()).unwrap_or(0.0) as usize,
                }),
                Some("rectangle") => {
                    let x1 = fields.get(1).and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
                    let y1 = fields.get(2).and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
                    let x2 = fields.get(3).and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
                    let y2 = fields.get(4).and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
                    let (time_start, time_end) = if vertical {
                        (y1 as f64, y2 as f64)
                    } else {
                        (x1 as f64, x2 as f64)
                    };
                    data.rects.push(GraphicTraceRect {
                        x1,
                        y1,
                        x2,
                        y2,
                        index: fields.get(5).and_then(|x| x.as_f64()).unwrap_or(0.0) as usize,
                        request: json_text(fields.get(6)),
                        chunk: json_text(fields.get(7)),
                        notes: json_text(fields.get(8)),
                        text: json_text(fields.get(9)),
                        time_start,
                        time_end,
                    });
                }
                _ => {}
            }
        }
        data
    }
}

#[derive(Debug, Clone, Default)]
pub struct GraphicTraceBoxView {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub index: usize,
    pub text: String,
    pub request: String,
    pub chunk: String,
    pub notes: String,
    pub time_start: f64,
    pub time_end: f64,
}

#[derive(Debug, Clone, Default)]
pub struct GraphicTraceLabelView {
    pub x: f32,
    pub y: f32,
    pub name: String,
    pub index: usize,
}

#[derive(Debug, Clone, Default)]
pub struct GraphicTraceLayout {
    pub vertical: bool,
    pub width: f32,
    pub height: f32,
    pub max_time: f64,
    pub time_scale: f32,
    pub boxes: Vec<GraphicTraceBoxView>,
    pub labels: Vec<GraphicTraceLabelView>,
}

pub fn layout_graphic_trace(
    data: &GraphicTraceData,
    vertical: bool,
    zoom: f32,
) -> GraphicTraceLayout {
    let base = if vertical { 0.15 } else { 0.20 };
    let time_scale = base * zoom.max(0.05);
    let max_x = data.rects.iter().map(|r| r.x2).fold(0.0f32, f32::max);
    let max_y = data.rects.iter().map(|r| r.y2).fold(0.0f32, f32::max);
    let mut layout = GraphicTraceLayout {
        vertical,
        max_time: data.max_time,
        time_scale,
        ..Default::default()
    };
    if vertical {
        layout.width = (data.labels.len() as f32 * 90.0 + 80.0)
            .max(max_x + 120.0)
            .max(600.0);
        layout.height = ((data.max_time * time_scale as f64) as f32 + 100.0)
            .max(max_y * time_scale + 100.0)
            .max(400.0);
    } else {
        layout.width = ((data.max_time * time_scale as f64) as f32 + 220.0).max(900.0);
        layout.height = ((data.labels.len() as f32 * 34.0 + 40.0).max(max_y + 40.0)).max(200.0);
    }
    for rect in &data.rects {
        layout.boxes.push(if vertical {
            GraphicTraceBoxView {
                x1: rect.x1 + 60.0,
                y1: (rect.y1 as f32 * time_scale) + 40.0,
                x2: rect.x2 + 60.0,
                y2: (rect.y2 as f32 * time_scale) + 40.0,
                index: rect.index,
                text: rect.text.clone(),
                request: rect.request.clone(),
                chunk: rect.chunk.clone(),
                notes: rect.notes.clone(),
                time_start: rect.time_start,
                time_end: rect.time_end,
            }
        } else {
            GraphicTraceBoxView {
                x1: (rect.x1 as f32 * time_scale) + 150.0,
                y1: rect.y1,
                x2: (rect.x2 as f32 * time_scale) + 150.0,
                y2: rect.y2,
                index: rect.index,
                text: rect.text.clone(),
                request: rect.request.clone(),
                chunk: rect.chunk.clone(),
                notes: rect.notes.clone(),
                time_start: rect.time_start,
                time_end: rect.time_end,
            }
        });
    }
    for label in &data.labels {
        layout.labels.push(if vertical {
            GraphicTraceLabelView {
                x: label.pos + 60.0,
                y: 8.0,
                name: label.name.clone(),
                index: label.index,
            }
        } else {
            GraphicTraceLabelView {
                x: 4.0,
                y: label.pos,
                name: label.name.clone(),
                index: label.index,
            }
        });
    }
    layout
}

pub fn graphic_trace_color(index: usize) -> (u8, u8, u8) {
    const COLORS: [(u8, u8, u8); 14] = [
        (0xff, 0x00, 0x00),
        (0x80, 0x80, 0xff),
        (0x00, 0x80, 0x00),
        (0xff, 0x80, 0x00),
        (0x7f, 0x7f, 0x7f),
        (0xff, 0x00, 0xff),
        (0xff, 0xff, 0x00),
        (0xc0, 0x80, 0x00),
        (0xf0, 0xf0, 0xf0),
        (0xc0, 0x00, 0xc0),
        (0x00, 0xc0, 0x50),
        (0xf0, 0xc0, 0xff),
        (0xc0, 0x58, 0x50),
        (0x50, 0xff, 0x25),
    ];
    COLORS[index % COLORS.len()]
}

fn gt_rgb(index: usize) -> (f32, f32, f32) {
    let (r, g, b) = graphic_trace_color(index);
    (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

fn gt_time_labels(layout: &GraphicTraceLayout) -> Vec<(f32, String)> {
    let mut out = Vec::new();
    let mut t = 0.0f64;
    while t <= layout.max_time {
        let pos = (t * layout.time_scale as f64) as f32;
        out.push((pos, format!("{:.3}", t / 1000.0)));
        t += 50.0;
    }
    out
}

fn gt_body(out: &mut String, layout: &GraphicTraceLayout, hide_text: bool, hide_grid: bool) {
    let h = layout.height;
    if !hide_grid {
        out.push_str("0.7 setgray 0.5 setlinewidth\n");
        for (pos, label) in gt_time_labels(layout) {
            if layout.vertical {
                let y = h - (pos + 40.0);
                out.push_str(&format!(
                    "newpath 0 {y:.1} moveto {:.1} {y:.1} lineto stroke\n",
                    layout.width
                ));
                out.push_str(&format!(
                    "0 setgray 6 {:.1} moveto ({}) show\n",
                    y - 2.0,
                    ps_escape(&label)
                ));
                out.push_str("0.7 setgray\n");
            } else {
                let x = pos + 150.0;
                out.push_str(&format!(
                    "newpath {x:.1} 0 moveto {x:.1} {h:.1} lineto stroke\n"
                ));
                out.push_str(&format!(
                    "0 setgray {x:.1} 2 moveto ({}) show\n",
                    ps_escape(&label)
                ));
                out.push_str("0.7 setgray\n");
            }
        }
        out.push_str("0 setgray 1 setlinewidth\n");
    }
    for label in &layout.labels {
        let (r, g, b) = gt_rgb(label.index);
        out.push_str(&format!(
            "{r:.3} {g:.3} {b:.3} setrgbcolor {:.1} {:.1} moveto ({}) show\n",
            label.x,
            h - label.y - 10.0,
            ps_escape(&label.name)
        ));
    }
    for b in &layout.boxes {
        let (r, g, bcol) = gt_rgb(b.index);
        let x = b.x1;
        let y = h - b.y2;
        let w = (b.x2 - b.x1).max(1.0);
        let bh = (b.y2 - b.y1).max(1.0);
        out.push_str(&format!(
            "{r:.3} {g:.3} {bcol:.3} setrgbcolor newpath {x:.1} {y:.1} moveto {w:.1} 0 rlineto 0 {bh:.1} rlineto -{w:.1} 0 rlineto closepath fill\n"
        ));
        out.push_str(&format!(
            "0 setgray 0.5 setlinewidth newpath {x:.1} {y:.1} moveto {w:.1} 0 rlineto 0 {bh:.1} rlineto -{w:.1} 0 rlineto closepath stroke 1 setlinewidth\n"
        ));
        if !hide_text && !b.text.is_empty() && w > 10.0 {
            out.push_str(&format!(
                "0 setgray {:.1} {:.1} moveto ({}) show\n",
                x + 2.0,
                y + bh - 9.0,
                ps_escape(&b.text)
            ));
        }
    }
}

pub fn graphic_trace_eps(layout: &GraphicTraceLayout, hide_text: bool, hide_grid: bool) -> String {
    let mut out = format!(
        "%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 {} {}\n/Helvetica findfont 8 scalefont setfont\n",
        layout.width.ceil(),
        layout.height.ceil()
    );
    gt_body(&mut out, layout, hide_text, hide_grid);
    out.push_str("%%EOF\n");
    out
}

pub fn graphic_trace_ps(layout: &GraphicTraceLayout, hide_text: bool, hide_grid: bool) -> String {
    let (page_w, page_h, count) = if layout.vertical {
        (
            layout.width,
            1000.0f32,
            (layout.height / 1000.0).ceil().max(1.0) as u32,
        )
    } else {
        (
            1400.0f32,
            layout.height,
            (layout.width / 1400.0).ceil().max(1.0) as u32,
        )
    };
    let mut out = format!(
        "%!PS-Adobe-2.0\n%%Pages: {count}\n%%EndComments\n/Helvetica findfont 8 scalefont setfont\n"
    );
    for page in 0..count {
        out.push_str(&format!("%%Page: {} {}\n", page + 1, page + 1));
        out.push_str("gsave\n");
        out.push_str(&format!("0 0 {page_w:.1} {page_h:.1} rectclip\n"));
        if layout.vertical {
            let yoff = page as f32 * page_h;
            let translate = yoff + page_h - layout.height;
            out.push_str(&format!("0 {translate:.1} translate\n"));
        } else {
            let xoff = page as f32 * page_w;
            out.push_str(&format!("-{xoff:.1} 0 translate\n"));
        }
        gt_body(&mut out, layout, hide_text, hide_grid);
        out.push_str("grestore showpage\n");
    }
    out.push_str("%%EOF\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_records() -> Vec<Json> {
        vec![
            Json::Arr(vec![
                Json::Int(0),
                Json::Str("info".into()),
                Json::Arr(vec![Json::Str("no match".into())]),
                Json::Arr(vec![
                    Json::Arr(vec![Json::Str("P1".into()), Json::Str("p1".into())]),
                    Json::Arr(vec![Json::Str("P2".into()), Json::Str("p2".into())]),
                ]),
            ]),
            Json::Arr(vec![
                Json::Int(1000),
                Json::Str("conflict-resolution".into()),
                Json::Bool(true),
                Json::Arr(vec![
                    Json::Arr(vec![Json::Int(0), Json::Float(1.2), Json::Float(0.0)]),
                    Json::Arr(vec![Json::Int(2), Json::Int(0)]),
                ]),
            ]),
        ]
    }

    #[test]
    fn parses_grid_and_filters_empty_columns() {
        let data = GridData::parse(&grid_records());
        assert_eq!(data.productions, vec!["P1", "P2"]);
        assert_eq!(data.times, vec!["1.000"]);
        assert_eq!(data.rows[0][0], GridCell::Selected("1.2".into()));
        assert_eq!(data.rows[1][0], GridCell::Mismatched("0".into()));
        assert_eq!(data.production_text[0], "p1");
        assert_eq!(data.why_not, vec!["no match".to_string()]);
        assert_eq!(data.visible_columns(false), vec![0]);
        assert_eq!(data.visible_columns(true), vec![0]);
    }

    #[test]
    fn writes_grid_eps_and_ps() {
        let data = GridData::parse(&grid_records());
        let eps = grid_eps(&data, false, "#44da22", "#fca31d", "#e1031e");
        assert!(eps.starts_with("%!PS-Adobe-3.0 EPSF-3.0"));
        assert!(eps.contains("P1"));
        assert!(eps.contains("1.000"));
        let ps = grid_ps(&data, false, "#44da22", "#fca31d", "#e1031e");
        assert!(ps.contains("%%Pages: 1"));
    }

    #[test]
    fn grid_ps_splits_wide_grids() {
        let mut data = GridData::default();
        data.productions = vec!["P".into()];
        data.times = (0..13).map(|i| format!("{i}.000")).collect();
        data.rows = vec![vec![GridCell::Blank; 13]];
        let ps = grid_ps(&data, false, "#000000", "#000000", "#000000");
        assert!(ps.contains("%%Pages: 2"));
        assert!(ps.contains("%%Page: 2 2"));
    }
    fn graph_data() -> crate::engine::ProductionGraph {
        use crate::engine::{
            ProductionGraph, ProductionGraphEdge, ProductionGraphMode, ProductionGraphNode,
            ProductionGraphPage,
        };
        ProductionGraph {
            mode: ProductionGraphMode::Frequencies,
            nodes: vec![
                ProductionGraphNode {
                    name: "A".into(),
                    text: "(a)".into(),
                    used: true,
                    starts: vec![0],
                    ends: Vec::new(),
                    utilities: Vec::new(),
                },
                ProductionGraphNode {
                    name: "B".into(),
                    text: "(b)".into(),
                    used: true,
                    starts: Vec::new(),
                    ends: vec![0],
                    utilities: Vec::new(),
                },
                ProductionGraphNode {
                    name: "UNUSED".into(),
                    text: "(u)".into(),
                    used: false,
                    starts: Vec::new(),
                    ends: Vec::new(),
                    utilities: Vec::new(),
                },
            ],
            edges: vec![
                ProductionGraphEdge {
                    from: "A".into(),
                    to: "B".into(),
                    count: 2,
                    page: 0,
                    dashed: false,
                    group: None,
                },
                ProductionGraphEdge {
                    from: "A".into(),
                    to: "UNUSED".into(),
                    count: 1,
                    page: 0,
                    dashed: true,
                    group: None,
                },
            ],
            pages: vec![ProductionGraphPage {
                start_ms: Some(100),
                end_ms: Some(200),
            }],
            utility_min: None,
            utility_max: None,
        }
    }

    #[test]
    fn writes_graph_dot_and_eps_and_hides_unused() {
        let data = graph_data();
        let layout = layout_graph_page(&data, 0, true);
        assert_eq!(
            layout
                .nodes
                .iter()
                .map(|n| n.name.as_str())
                .collect::<Vec<_>>(),
            vec!["A", "B"]
        );
        let dot = graph_dot(&layout);
        assert!(dot.contains("\"A\" -> \"B\""));
        assert!(dot.contains("label=\"2\""));
        assert!(!dot.contains("UNUSED"));
        let eps = graph_eps(&layout);
        assert!(eps.starts_with("%!PS-Adobe-3.0 EPSF-3.0"));
        assert!(eps.contains("(A) show"));
        let full = layout_graph_page(&data, 0, false);
        let dot = graph_dot(&full);
        assert!(dot.contains("style=dashed"));
        assert!(dot.contains("UNUSED"));
    }

    fn graphic_records(vertical: bool) -> Vec<Json> {
        let rect = if vertical {
            vec![
                Json::Str("rectangle".into()),
                Json::Int(0),
                Json::Int(100),
                Json::Int(20),
                Json::Int(300),
                Json::Int(1),
                Json::Str("goal".into()),
                Json::Str("chunk".into()),
                Json::Str("note".into()),
                Json::Str("chunk".into()),
                Json::Str("goal".into()),
            ]
        } else {
            vec![
                Json::Str("rectangle".into()),
                Json::Int(100),
                Json::Int(0),
                Json::Int(300),
                Json::Int(20),
                Json::Int(1),
                Json::Str("goal".into()),
                Json::Str("chunk".into()),
                Json::Str("note".into()),
                Json::Str("chunk".into()),
                Json::Str("goal".into()),
            ]
        };
        vec![
            Json::Arr(vec![Json::Str("size".into()), Json::Int(500)]),
            Json::Arr(vec![
                Json::Str("label".into()),
                Json::Str("goal".into()),
                Json::Int(10),
                Json::Int(1),
            ]),
            Json::Arr(rect),
        ]
    }

    #[test]
    fn parses_and_exports_graphic_traces() {
        let data = GraphicTraceData::parse(&graphic_records(false), false);
        assert_eq!(data.rects.len(), 1);
        assert_eq!(data.rects[0].request, "goal");
        assert_eq!(data.rects[0].chunk, "chunk");
        assert_eq!(data.rects[0].notes, "note");
        assert_eq!(data.rects[0].time_start, 100.0);
        let layout = layout_graphic_trace(&data, false, 2.0);
        assert!(layout.width >= 900.0);
        let eps = graphic_trace_eps(&layout, false, false);
        assert!(eps.starts_with("%!PS-Adobe-3.0 EPSF-3.0"));
        assert!(eps.contains("(goal) show"));
        assert!(eps.contains("(chunk) show"));
        let hidden = graphic_trace_eps(&layout, true, true);
        assert!(!hidden.contains("(chunk) show"));
        assert!(!hidden.contains("0.7 setgray"));
        let ps = graphic_trace_ps(&graphic_trace_layout_wide(), false, true);
        assert!(ps.contains("%%Pages: 2"));
        assert!(ps.contains("%%Page: 2 2"));
    }

    fn graphic_trace_layout_wide() -> GraphicTraceLayout {
        let mut data = GraphicTraceData::parse(&graphic_records(false), false);
        data.max_time = 5000.0;
        layout_graphic_trace(&data, false, 2.0)
    }
}
