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

#[derive(Debug, Clone, Default)]
pub struct GraphLayout {
    pub nodes: Vec<String>,
    pub edges: Vec<(String, String, u32)>,
    pub positions: HashMap<String, (f32, f32)>,
    pub width: f32,
    pub height: f32,
}

pub const GRAPH_BOX_W: f32 = 130.0;
pub const GRAPH_BOX_H: f32 = 30.0;

pub fn layout_graph(
    nodes: &[String],
    edges: &[(String, String, u32)],
    hide_unused: bool,
) -> GraphLayout {
    let used: Vec<String> = if hide_unused {
        nodes
            .iter()
            .filter(|node| edges.iter().any(|(a, b, _)| a == *node || b == *node))
            .cloned()
            .collect()
    } else {
        nodes.to_vec()
    };
    let visible_edges: Vec<(String, String, u32)> = edges
        .iter()
        .filter(|(a, b, _)| used.iter().any(|n| n == a) && used.iter().any(|n| n == b))
        .cloned()
        .collect();

    let mut layer: HashMap<&str, usize> = used.iter().map(|n| (n.as_str(), 0usize)).collect();
    for _ in 0..used.len().min(20) {
        let mut changed = false;
        for (a, b, _) in &visible_edges {
            let la = layer.get(a.as_str()).copied().unwrap_or(0);
            let lb = layer.get(b.as_str()).copied().unwrap_or(0);
            if lb <= la {
                layer.insert(b.as_str(), (la + 1).min(used.len()));
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let max_layer = *layer.values().max().unwrap_or(&0);
    let mut by_layer: Vec<Vec<&String>> = vec![Vec::new(); max_layer + 1];
    for node in &used {
        by_layer[layer.get(node.as_str()).copied().unwrap_or(0)].push(node);
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
                (*node).clone(),
                (
                    20.0 + xi as f32 * (GRAPH_BOX_W + h_gap),
                    20.0 + li as f32 * (GRAPH_BOX_H + v_gap),
                ),
            );
        }
    }
    GraphLayout {
        nodes: used,
        edges: visible_edges,
        positions,
        width,
        height,
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
    let mut out = String::from("digraph production_history {\n");
    for node in &layout.nodes {
        out.push_str(&format!("  \"{}\" ;\n", dot_escape(node)));
    }
    for (a, b, count) in &layout.edges {
        if *count > 1 {
            out.push_str(&format!(
                "  \"{}\" -> \"{}\" [label=\"{}\"] ;\n",
                dot_escape(a),
                dot_escape(b),
                count
            ));
        } else {
            out.push_str(&format!(
                "  \"{}\" -> \"{}\" ;\n",
                dot_escape(a),
                dot_escape(b)
            ));
        }
    }
    out.push_str("}\n");
    out
}

pub fn graph_eps(layout: &GraphLayout) -> String {
    let mut out = format!(
        "%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 {} {}\n",
        layout.width.ceil(),
        layout.height.ceil()
    );
    out.push_str("/Helvetica findfont 8 scalefont setfont\n");
    for (a, b, _) in &layout.edges {
        let (Some(&(ax, ay)), Some(&(bx, by))) = (layout.positions.get(a), layout.positions.get(b))
        else {
            continue;
        };
        out.push_str(&format!(
            "0.5 setgray newpath {:.1} {:.1} moveto {:.1} {:.1} lineto stroke\n",
            ax + GRAPH_BOX_W / 2.0,
            layout.height - (ay + GRAPH_BOX_H),
            bx + GRAPH_BOX_W / 2.0,
            layout.height - by
        ));
    }
    for node in &layout.nodes {
        let Some(&(x, y)) = layout.positions.get(node) else {
            continue;
        };
        let py = layout.height - (y + GRAPH_BOX_H);
        out.push_str(&format!(
            "0.95 setgray newpath {x:.1} {py:.1} moveto {GRAPH_BOX_W:.1} 0 rlineto 0 {GRAPH_BOX_H:.1} rlineto -{GRAPH_BOX_W:.1} 0 rlineto closepath fill\n"
        ));
        out.push_str(&format!(
            "0 setgray newpath {x:.1} {py:.1} moveto {GRAPH_BOX_W:.1} 0 rlineto 0 {GRAPH_BOX_H:.1} rlineto -{GRAPH_BOX_W:.1} 0 rlineto closepath stroke\n"
        ));
        out.push_str(&format!(
            "{:.1} {:.1} moveto ({}) show\n",
            x + 4.0,
            py + 10.0,
            ps_escape(node)
        ));
    }
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
    #[test]
    fn writes_graph_dot_and_eps_and_hides_unused() {
        let nodes = vec!["A".to_string(), "B".to_string(), "UNUSED".to_string()];
        let edges = vec![("A".to_string(), "B".to_string(), 2u32)];
        let layout = layout_graph(&nodes, &edges, true);
        assert_eq!(layout.nodes, vec!["A", "B"]);
        let dot = graph_dot(&layout);
        assert!(dot.contains("\"A\" -> \"B\""));
        assert!(!dot.contains("UNUSED"));
        let eps = graph_eps(&layout);
        assert!(eps.starts_with("%!PS-Adobe-3.0 EPSF-3.0"));
    }
}
