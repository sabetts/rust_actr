use std::collections::{HashMap, HashSet};

use crate::json::Json;

use super::{Engine, PHistRecord, PHistTag};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductionGraphMode {
    All,
    Frequencies,
    Cycles,
    UniqueCycles,
    Runs,
    UniqueRuns,
    Utilities,
}

impl Default for ProductionGraphMode {
    fn default() -> Self {
        Self::All
    }
}

impl ProductionGraphMode {
    pub fn key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Frequencies => "freq",
            Self::Cycles => "cycles",
            Self::UniqueCycles => "u-cycles",
            Self::Runs => "runs",
            Self::UniqueRuns => "u-runs",
            Self::Utilities => "utility",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All Transitions",
            Self::Frequencies => "Frequencies",
            Self::Cycles => "Cycles",
            Self::UniqueCycles => "Unique Cycles",
            Self::Runs => "Runs",
            Self::UniqueRuns => "Unique Runs",
            Self::Utilities => "Utilities",
        }
    }

    pub fn command(self) -> &'static str {
        match self {
            Self::All => "production-graph-all",
            Self::Frequencies => "production-graph-freq",
            Self::Cycles => "production-graph-cycles",
            Self::UniqueCycles => "production-graph-u-cycles",
            Self::Runs => "production-graph-runs",
            Self::UniqueRuns => "production-graph-u-runs",
            Self::Utilities => "production-graph-utility",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "all" => Self::All,
            "freq" | "frequencies" => Self::Frequencies,
            "cycles" => Self::Cycles,
            "u-cycles" | "unique-cycles" => Self::UniqueCycles,
            "runs" => Self::Runs,
            "u-runs" | "unique-runs" => Self::UniqueRuns,
            "utility" | "utilities" => Self::Utilities,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProductionGraphNode {
    pub name: String,
    pub text: String,
    pub used: bool,
    pub starts: Vec<u32>,
    pub ends: Vec<u32>,
    pub utilities: Vec<(u32, f64)>,
}

#[derive(Debug, Clone)]
pub struct ProductionGraphEdge {
    pub from: String,
    pub to: String,
    pub count: u32,
    pub page: u32,
    pub dashed: bool,
    pub group: Option<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct ProductionGraphPage {
    pub start_ms: Option<i64>,
    pub end_ms: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct ProductionGraph {
    pub mode: ProductionGraphMode,
    pub nodes: Vec<ProductionGraphNode>,
    pub edges: Vec<ProductionGraphEdge>,
    pub pages: Vec<ProductionGraphPage>,
    pub utility_min: Option<f64>,
    pub utility_max: Option<f64>,
}

impl ProductionGraph {
    pub fn to_json(&self) -> Json {
        let nodes = self
            .nodes
            .iter()
            .map(|n| {
                Json::Obj(vec![
                    ("name".into(), Json::Str(n.name.clone())),
                    ("text".into(), Json::Str(n.text.clone())),
                    ("used".into(), Json::Bool(n.used)),
                    (
                        "starts".into(),
                        Json::Arr(n.starts.iter().map(|p| Json::Int(*p as i64)).collect()),
                    ),
                    (
                        "ends".into(),
                        Json::Arr(n.ends.iter().map(|p| Json::Int(*p as i64)).collect()),
                    ),
                    (
                        "utilities".into(),
                        Json::Arr(
                            n.utilities
                                .iter()
                                .map(|(p, u)| {
                                    Json::Arr(vec![Json::Int(*p as i64), Json::Float(*u)])
                                })
                                .collect(),
                        ),
                    ),
                ])
            })
            .collect();
        let edges = self
            .edges
            .iter()
            .map(|e| {
                Json::Obj(vec![
                    ("from".into(), Json::Str(e.from.clone())),
                    ("to".into(), Json::Str(e.to.clone())),
                    ("count".into(), Json::Int(e.count as i64)),
                    ("page".into(), Json::Int(e.page as i64)),
                    ("dashed".into(), Json::Bool(e.dashed)),
                    (
                        "group".into(),
                        e.group.map(|g| Json::Int(g as i64)).unwrap_or(Json::Null),
                    ),
                ])
            })
            .collect();
        let pages = self
            .pages
            .iter()
            .map(|p| {
                Json::Obj(vec![
                    (
                        "start_ms".into(),
                        p.start_ms.map(Json::Int).unwrap_or(Json::Null),
                    ),
                    (
                        "end_ms".into(),
                        p.end_ms.map(Json::Int).unwrap_or(Json::Null),
                    ),
                ])
            })
            .collect();
        Json::Obj(vec![
            ("mode".into(), Json::Str(self.mode.key().into())),
            ("nodes".into(), Json::Arr(nodes)),
            ("edges".into(), Json::Arr(edges)),
            ("pages".into(), Json::Arr(pages)),
            (
                "utility_min".into(),
                self.utility_min.map(Json::Float).unwrap_or(Json::Null),
            ),
            (
                "utility_max".into(),
                self.utility_max.map(Json::Float).unwrap_or(Json::Null),
            ),
        ])
    }

    pub fn from_json(value: &Json) -> Option<Self> {
        let mode = ProductionGraphMode::from_key(value.get("mode")?.as_str()?)?;
        let nodes = json_array(value.get("nodes")?)?
            .iter()
            .map(|v| {
                Some(ProductionGraphNode {
                    name: v.get("name")?.as_str()?.to_string(),
                    text: v
                        .get("text")
                        .and_then(Json::as_str)
                        .unwrap_or("")
                        .to_string(),
                    used: matches!(v.get("used"), Some(Json::Bool(true))),
                    starts: int_list(v.get("starts")),
                    ends: int_list(v.get("ends")),
                    utilities: v
                        .get("utilities")
                        .and_then(json_array_opt)
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(|row| {
                                    let row = json_array(row)?;
                                    Some((row.first()?.as_f64()? as u32, row.get(1)?.as_f64()?))
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let edges = json_array(value.get("edges")?)?
            .iter()
            .map(|v| {
                Some(ProductionGraphEdge {
                    from: v.get("from")?.as_str()?.to_string(),
                    to: v.get("to")?.as_str()?.to_string(),
                    count: v.get("count")?.as_f64()? as u32,
                    page: v.get("page")?.as_f64()? as u32,
                    dashed: matches!(v.get("dashed"), Some(Json::Bool(true))),
                    group: v.get("group").and_then(Json::as_f64).map(|g| g as u32),
                })
            })
            .collect::<Option<Vec<_>>>()?;
        let pages = json_array(value.get("pages")?)?
            .iter()
            .map(|v| {
                Some(ProductionGraphPage {
                    start_ms: v.get("start_ms").and_then(Json::as_f64).map(|x| x as i64),
                    end_ms: v.get("end_ms").and_then(Json::as_f64).map(|x| x as i64),
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(ProductionGraph {
            mode,
            nodes,
            edges,
            pages,
            utility_min: value.get("utility_min").and_then(Json::as_f64),
            utility_max: value.get("utility_max").and_then(Json::as_f64),
        })
    }
}

fn json_array(value: &Json) -> Option<&Vec<Json>> {
    match value {
        Json::Arr(items) => Some(items),
        _ => None,
    }
}

fn json_array_opt(value: &Json) -> Option<&Vec<Json>> {
    json_array(value)
}

fn int_list(value: Option<&Json>) -> Vec<u32> {
    value
        .and_then(json_array_opt)
        .map(|items| {
            items
                .iter()
                .filter_map(|x| x.as_f64().map(|v| v as u32))
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Default, Clone)]
struct PageAcc {
    start_ms: Option<i64>,
    end_ms: Option<i64>,
    start_node: Option<String>,
    end_node: Option<String>,
    utilities: Vec<(String, f64)>,
}

fn page_acc(pages: &mut Vec<PageAcc>, page: u32) -> &mut PageAcc {
    while pages.len() <= page as usize {
        pages.push(PageAcc::default());
    }
    &mut pages[page as usize]
}

fn note_time(page: &mut PageAcc, time: i64) {
    page.start_ms = Some(page.start_ms.map_or(time, |x| x.min(time)));
    page.end_ms = Some(page.end_ms.map_or(time, |x| x.max(time)));
}

fn merge_page(into: &mut PageAcc, from: PageAcc) {
    if let Some(t) = from.start_ms {
        into.start_ms = Some(into.start_ms.map_or(t, |x| x.min(t)));
    }
    if let Some(t) = from.end_ms {
        into.end_ms = Some(into.end_ms.map_or(t, |x| x.max(t)));
    }
    if into.start_node.is_none() {
        into.start_node = from.start_node;
    }
    if from.end_node.is_some() {
        into.end_node = from.end_node;
    }
    into.utilities.extend(from.utilities);
}

struct Builder {
    nodes: Vec<ProductionGraphNode>,
    index: HashMap<String, usize>,
    edges: Vec<ProductionGraphEdge>,
    solid: HashMap<(String, String, u32), usize>,
    dashed: HashSet<(String, String, u32)>,
}

impl Builder {
    fn new(productions: &[String], texts: &[String]) -> Self {
        let nodes = productions
            .iter()
            .enumerate()
            .map(|(i, name)| ProductionGraphNode {
                name: name.clone(),
                text: texts.get(i).cloned().unwrap_or_default(),
                ..Default::default()
            })
            .collect::<Vec<_>>();
        let index = nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.name.clone(), i))
            .collect();
        Self {
            nodes,
            index,
            edges: Vec::new(),
            solid: HashMap::new(),
            dashed: HashSet::new(),
        }
    }

    fn used(&mut self, name: &str) {
        if let Some(i) = self.index.get(name).copied() {
            self.nodes[i].used = true;
        }
    }

    fn solid(&mut self, from: &str, to: &str, page: u32) {
        self.used(from);
        self.used(to);
        let key = (from.to_string(), to.to_string(), page);
        if let Some(i) = self.solid.get(&key).copied() {
            self.edges[i].count += 1;
            return;
        }
        let i = self.edges.len();
        self.solid.insert(key, i);
        self.edges.push(ProductionGraphEdge {
            from: from.to_string(),
            to: to.to_string(),
            count: 1,
            page,
            dashed: false,
            group: None,
        });
    }

    fn dashed(&mut self, from: &str, to: &str, page: u32) {
        let key = (from.to_string(), to.to_string(), page);
        if !self.dashed.insert(key) {
            return;
        }
        self.edges.push(ProductionGraphEdge {
            from: from.to_string(),
            to: to.to_string(),
            count: 1,
            page,
            dashed: true,
            group: None,
        });
    }

    fn mark(&mut self, name: &str, page: u32, start: bool) {
        let Some(i) = self.index.get(name).copied() else {
            return;
        };
        self.nodes[i].used = true;
        let list = if start {
            &mut self.nodes[i].starts
        } else {
            &mut self.nodes[i].ends
        };
        if !list.contains(&page) {
            list.push(page);
        }
    }

    fn finish(
        mut self,
        mode: ProductionGraphMode,
        pages: Vec<PageAcc>,
        mut utility_min: Option<f64>,
        mut utility_max: Option<f64>,
    ) -> ProductionGraph {
        for (page, acc) in pages.iter().enumerate() {
            if let Some(name) = &acc.start_node {
                self.mark(name, page as u32, true);
            }
            if let Some(name) = &acc.end_node {
                self.mark(name, page as u32, false);
            }
            for (name, value) in &acc.utilities {
                if let Some(i) = self.index.get(name).copied() {
                    self.nodes[i].used = true;
                    self.nodes[i].utilities.push((page as u32, *value));
                    utility_min = Some(utility_min.map_or(*value, |x| x.min(*value)));
                    utility_max = Some(utility_max.map_or(*value, |x| x.max(*value)));
                }
            }
        }
        if let (Some(min), Some(max)) = (utility_min, utility_max) {
            if min == max {
                utility_min = Some(min - 1.0);
                utility_max = Some(max + 1.0);
            }
        }
        ProductionGraph {
            mode,
            nodes: self.nodes,
            edges: self.edges,
            pages: pages
                .into_iter()
                .map(|p| ProductionGraphPage {
                    start_ms: p.start_ms,
                    end_ms: p.end_ms,
                })
                .collect(),
            utility_min,
            utility_max,
        }
    }
}

pub fn analyze(
    mode: ProductionGraphMode,
    productions: &[String],
    texts: &[String],
    records: &[PHistRecord],
) -> ProductionGraph {
    match mode {
        ProductionGraphMode::All | ProductionGraphMode::Frequencies => {
            analyze_simple(mode, productions, texts, records)
        }
        ProductionGraphMode::Cycles | ProductionGraphMode::UniqueCycles => {
            analyze_cycles(mode, productions, texts, records)
        }
        ProductionGraphMode::Runs | ProductionGraphMode::UniqueRuns => {
            analyze_runs(mode, productions, texts, records)
        }
        ProductionGraphMode::Utilities => analyze_utilities(productions, texts, records),
    }
}

fn analyze_simple(
    mode: ProductionGraphMode,
    productions: &[String],
    texts: &[String],
    records: &[PHistRecord],
) -> ProductionGraph {
    let mut b = Builder::new(productions, texts);
    let mut pages = Vec::new();
    let mut prev: Option<String> = None;
    let mut first: Option<String> = None;
    let mut last: Option<String> = None;
    for r in records {
        match r.tag {
            PHistTag::Start => prev = None,
            PHistTag::Conflict => {
                note_time(page_acc(&mut pages, 0), r.time_ms);
                if let Some(p) = &prev {
                    for m in &r.matched {
                        if Some(m) != r.selected.as_ref() {
                            b.dashed(p, m, 0);
                        }
                    }
                }
                if let Some(sel) = &r.selected {
                    if let Some(p) = &prev {
                        b.solid(p, sel, 0);
                    }
                    if first.is_none() {
                        first = Some(sel.clone());
                    }
                    last = Some(sel.clone());
                    prev = Some(sel.clone());
                }
            }
            _ => {}
        }
    }
    if pages.is_empty() {
        pages.push(PageAcc::default());
    }
    pages[0].start_node = first;
    pages[0].end_node = last;
    b.finish(mode, pages, None, None)
}

fn group_sequences(sequences: &[Vec<String>]) -> (Vec<u32>, Vec<Vec<String>>) {
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut mapping = Vec::with_capacity(sequences.len());
    for sequence in sequences {
        let group = match groups.iter().position(|g| g == sequence) {
            Some(i) => i,
            None => {
                groups.push(sequence.clone());
                groups.len() - 1
            }
        };
        mapping.push(group as u32);
    }
    (mapping, groups)
}

fn merge_edges(edges: Vec<ProductionGraphEdge>) -> Vec<ProductionGraphEdge> {
    let mut out: Vec<ProductionGraphEdge> = Vec::new();
    let mut solid: HashMap<(String, String, u32), usize> = HashMap::new();
    let mut dashed: HashSet<(String, String, u32)> = HashSet::new();
    for edge in edges {
        let key = (edge.from.clone(), edge.to.clone(), edge.page);
        if edge.dashed {
            if dashed.insert(key) {
                out.push(edge);
            }
        } else if let Some(i) = solid.get(&key).copied() {
            out[i].count += edge.count;
        } else {
            solid.insert(key, out.len());
            out.push(edge);
        }
    }
    out
}

fn remap(b: &mut Builder, pages: Vec<PageAcc>, mapping: &[u32], group_edges: bool) -> Vec<PageAcc> {
    let mut out =
        vec![PageAcc::default(); mapping.iter().copied().max().map_or(0, |m| m as usize + 1)];
    for (old, acc) in pages.into_iter().enumerate() {
        let new = mapping.get(old).copied().unwrap_or(0) as usize;
        if out.len() <= new {
            out.resize(new + 1, PageAcc::default());
        }
        merge_page(&mut out[new], acc);
    }
    for edge in &mut b.edges {
        edge.page = mapping.get(edge.page as usize).copied().unwrap_or(0);
        if group_edges {
            edge.group = Some(edge.page);
        }
    }
    b.edges = merge_edges(std::mem::take(&mut b.edges));
    if out.is_empty() {
        out.push(PageAcc::default());
    }
    out
}

fn analyze_cycles(
    mode: ProductionGraphMode,
    productions: &[String],
    texts: &[String],
    records: &[PHistRecord],
) -> ProductionGraph {
    let mut b = Builder::new(productions, texts);
    let mut pages = Vec::new();
    let mut sequences = Vec::new();
    let mut current = Vec::new();
    let mut prev: Option<String> = None;
    let mut page = 0u32;
    for r in records {
        match r.tag {
            PHistTag::Start => {
                prev = None;
                current.clear();
            }
            PHistTag::Conflict => {
                if let Some(p) = &prev {
                    for m in &r.matched {
                        if Some(m) != r.selected.as_ref() {
                            b.dashed(p, m, page);
                        }
                    }
                }
                if let Some(sel) = &r.selected {
                    let acc = page_acc(&mut pages, page);
                    if acc.start_ms.is_none() {
                        acc.start_ms = Some(r.time_ms);
                    }
                    note_time(acc, r.time_ms);
                    if let Some(p) = &prev {
                        b.solid(p, sel, page);
                    }
                    if let Some(pos) = current.iter().position(|n| n == sel) {
                        let sequence = current.clone();
                        let acc = page_acc(&mut pages, page);
                        acc.start_node = sequence.first().cloned();
                        acc.end_node = Some(sel.clone());
                        acc.end_ms = Some(r.time_ms);
                        sequences.push(sequence);
                        current.truncate(pos);
                        current.clear();
                        page += 1;
                    } else {
                        current.push(sel.clone());
                        let acc = page_acc(&mut pages, page);
                        acc.end_ms = Some(r.time_ms);
                    }
                    prev = Some(sel.clone());
                }
            }
            _ => {}
        }
    }
    if !current.is_empty() {
        let acc = page_acc(&mut pages, page);
        acc.start_node = current.first().cloned();
        acc.end_node = current.last().cloned();
        sequences.push(current);
    }
    if pages.is_empty() {
        pages.push(PageAcc::default());
    }
    if mode == ProductionGraphMode::UniqueCycles {
        let (mapping, _) = group_sequences(&sequences);
        let pages = remap(&mut b, pages, &mapping, true);
        b.finish(mode, pages, None, None)
    } else {
        b.finish(mode, pages, None, None)
    }
}

fn analyze_runs(
    mode: ProductionGraphMode,
    productions: &[String],
    texts: &[String],
    records: &[PHistRecord],
) -> ProductionGraph {
    let mut b = Builder::new(productions, texts);
    let mut pages = Vec::new();
    let mut sequences = Vec::new();
    let mut current = Vec::new();
    let mut prev: Option<String> = None;
    let mut page = 0u32;
    for r in records {
        match r.tag {
            PHistTag::Start => {
                prev = None;
                current.clear();
            }
            PHistTag::Conflict => {
                if let Some(p) = &prev {
                    for m in &r.matched {
                        if Some(m) != r.selected.as_ref() {
                            b.dashed(p, m, page);
                        }
                    }
                }
                if let Some(sel) = &r.selected {
                    let acc = page_acc(&mut pages, page);
                    if acc.start_ms.is_none() {
                        acc.start_ms = Some(r.time_ms);
                    }
                    note_time(acc, r.time_ms);
                    if let Some(p) = &prev {
                        b.solid(p, sel, page);
                    }
                    current.push(sel.clone());
                    acc.end_ms = Some(r.time_ms);
                    prev = Some(sel.clone());
                }
            }
            PHistTag::Stop => {
                if !current.is_empty() {
                    let acc = page_acc(&mut pages, page);
                    acc.start_node = current.first().cloned();
                    acc.end_node = current.last().cloned();
                    note_time(acc, r.time_ms);
                    sequences.push(current.clone());
                    current.clear();
                    prev = None;
                    page += 1;
                }
            }
            PHistTag::Reward => {}
        }
    }
    if !current.is_empty() {
        let acc = page_acc(&mut pages, page);
        acc.start_node = current.first().cloned();
        acc.end_node = current.last().cloned();
        sequences.push(current);
    }
    if pages.is_empty() {
        pages.push(PageAcc::default());
    }
    if mode == ProductionGraphMode::UniqueRuns {
        let (mapping, _) = group_sequences(&sequences);
        let pages = remap(&mut b, pages, &mapping, true);
        b.finish(mode, pages, None, None)
    } else {
        b.finish(mode, pages, None, None)
    }
}

fn utilities_from(info: &[Vec<Json>]) -> Vec<(String, f64)> {
    info.iter()
        .filter_map(|row| {
            let name = row.first()?.as_str()?.to_string();
            let value = row
                .get(1)
                .and_then(Json::as_f64)
                .or_else(|| row.get(2).and_then(Json::as_f64))?;
            Some((name, value))
        })
        .collect()
}

fn analyze_utilities(
    productions: &[String],
    texts: &[String],
    records: &[PHistRecord],
) -> ProductionGraph {
    let mut b = Builder::new(productions, texts);
    let mut pages = Vec::new();
    let mut current = Vec::new();
    let mut prev: Option<String> = None;
    let mut page = 0u32;
    for r in records {
        match r.tag {
            PHistTag::Start => {
                prev = None;
                current.clear();
            }
            PHistTag::Conflict => {
                if let Some(p) = &prev {
                    for m in &r.matched {
                        if Some(m) != r.selected.as_ref() {
                            b.dashed(p, m, page);
                        }
                    }
                }
                if let Some(sel) = &r.selected {
                    let acc = page_acc(&mut pages, page);
                    if acc.start_ms.is_none() {
                        acc.start_ms = Some(r.time_ms);
                    }
                    note_time(acc, r.time_ms);
                    if let Some(p) = &prev {
                        b.solid(p, sel, page);
                    }
                    current.push(sel.clone());
                    acc.end_ms = Some(r.time_ms);
                    prev = Some(sel.clone());
                }
            }
            PHistTag::Reward => {
                let target = if current.is_empty() && page > 0 {
                    page - 1
                } else {
                    page
                };
                let utils = utilities_from(&r.info);
                if !utils.is_empty() {
                    let acc = page_acc(&mut pages, target);
                    note_time(acc, r.time_ms);
                    acc.utilities = utils;
                }
            }
            PHistTag::Stop => {
                if !current.is_empty() {
                    let acc = page_acc(&mut pages, page);
                    if acc.utilities.is_empty() {
                        acc.utilities = utilities_from(&r.info);
                    }
                    acc.start_node = current.first().cloned();
                    acc.end_node = current.last().cloned();
                    note_time(acc, r.time_ms);
                    current.clear();
                    prev = None;
                    page += 1;
                }
            }
        }
    }
    if !current.is_empty() {
        let acc = page_acc(&mut pages, page);
        acc.start_node = current.first().cloned();
        acc.end_node = current.last().cloned();
    }
    if pages.is_empty() {
        pages.push(PageAcc::default());
    }
    b.finish(ProductionGraphMode::Utilities, pages, None, None)
}

impl Engine {
    pub fn production_graph_data(&self, mode: ProductionGraphMode) -> ProductionGraph {
        let productions = self.all_production_names();
        let texts = productions
            .iter()
            .map(|name| self.printed_production(name).unwrap_or_default())
            .collect::<Vec<_>>();
        let records = self.production_history();
        analyze(mode, &productions, &texts, &records)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(tag: PHistTag, time: i64, selected: Option<&str>, matched: &[&str]) -> PHistRecord {
        PHistRecord {
            tag,
            time_ms: time,
            selected: selected.map(str::to_string),
            matched: matched.iter().map(|s| s.to_string()).collect(),
            mismatched: Vec::new(),
            info: Vec::new(),
            reward: None,
        }
    }

    fn reward(time: i64, rows: &[(&str, f64)]) -> PHistRecord {
        PHistRecord {
            tag: PHistTag::Reward,
            time_ms: time,
            selected: None,
            matched: Vec::new(),
            mismatched: Vec::new(),
            info: rows
                .iter()
                .map(|(name, value)| {
                    vec![
                        Json::Str((*name).into()),
                        Json::Float(*value),
                        Json::Float(*value),
                    ]
                })
                .collect(),
            reward: Some(0.0),
        }
    }

    fn names() -> Vec<String> {
        vec!["A".into(), "B".into(), "C".into()]
    }

    fn texts() -> Vec<String> {
        vec!["(a)".into(), "(b)".into(), "(c)".into()]
    }

    fn node<'a>(graph: &'a ProductionGraph, name: &str) -> &'a ProductionGraphNode {
        graph.nodes.iter().find(|n| n.name == name).unwrap()
    }

    #[test]
    fn all_has_solid_and_dashed_edges() {
        let records = vec![
            rec(PHistTag::Start, 0, None, &[]),
            rec(PHistTag::Conflict, 100, Some("A"), &["A"]),
            rec(PHistTag::Conflict, 200, Some("B"), &["B", "C"]),
            rec(PHistTag::Stop, 300, None, &[]),
        ];
        let graph = analyze(ProductionGraphMode::All, &names(), &texts(), &records);
        assert_eq!(graph.pages.len(), 1);
        assert!(graph
            .edges
            .iter()
            .any(|e| e.from == "A" && e.to == "B" && !e.dashed));
        assert!(graph
            .edges
            .iter()
            .any(|e| e.from == "A" && e.to == "C" && e.dashed));
        assert!(node(&graph, "A").used);
        assert!(node(&graph, "B").used);
        assert!(!node(&graph, "C").used);
        assert_eq!(node(&graph, "A").starts, vec![0]);
        assert_eq!(node(&graph, "B").ends, vec![0]);
        assert_eq!(node(&graph, "C").text, "(c)");
    }

    #[test]
    fn frequencies_keep_counts() {
        let records = vec![
            rec(PHistTag::Start, 0, None, &[]),
            rec(PHistTag::Conflict, 100, Some("A"), &["A"]),
            rec(PHistTag::Conflict, 200, Some("B"), &["B"]),
            rec(PHistTag::Conflict, 300, Some("A"), &["A"]),
            rec(PHistTag::Conflict, 400, Some("B"), &["B"]),
        ];
        let graph = analyze(
            ProductionGraphMode::Frequencies,
            &names(),
            &texts(),
            &records,
        );
        let ab = graph
            .edges
            .iter()
            .find(|e| e.from == "A" && e.to == "B")
            .unwrap();
        let ba = graph
            .edges
            .iter()
            .find(|e| e.from == "B" && e.to == "A")
            .unwrap();
        assert_eq!(ab.count, 2);
        assert_eq!(ba.count, 1);
    }

    #[test]
    fn cycles_page_and_group_repeats() {
        let records = vec![
            rec(PHistTag::Start, 0, None, &[]),
            rec(PHistTag::Conflict, 100, Some("A"), &["A"]),
            rec(PHistTag::Conflict, 200, Some("B"), &["B"]),
            rec(PHistTag::Conflict, 300, Some("A"), &["A"]),
            rec(PHistTag::Conflict, 400, Some("A"), &["A"]),
            rec(PHistTag::Conflict, 500, Some("B"), &["B"]),
            rec(PHistTag::Conflict, 600, Some("A"), &["A"]),
        ];
        let cycles = analyze(ProductionGraphMode::Cycles, &names(), &texts(), &records);
        assert_eq!(cycles.pages.len(), 2);
        let unique = analyze(
            ProductionGraphMode::UniqueCycles,
            &names(),
            &texts(),
            &records,
        );
        assert_eq!(unique.pages.len(), 1);
        let ab = unique
            .edges
            .iter()
            .find(|e| e.from == "A" && e.to == "B")
            .unwrap();
        assert_eq!(ab.count, 2);
        assert_eq!(ab.group, Some(0));
    }

    #[test]
    fn runs_split_on_stop_and_group_repeats() {
        let records = vec![
            rec(PHistTag::Start, 0, None, &[]),
            rec(PHistTag::Conflict, 100, Some("A"), &["A", "C"]),
            rec(PHistTag::Conflict, 200, Some("B"), &["B"]),
            rec(PHistTag::Stop, 300, None, &[]),
            rec(PHistTag::Start, 400, None, &[]),
            rec(PHistTag::Conflict, 500, Some("A"), &["A", "C"]),
            rec(PHistTag::Conflict, 600, Some("B"), &["B"]),
            rec(PHistTag::Stop, 700, None, &[]),
        ];
        let runs = analyze(ProductionGraphMode::Runs, &names(), &texts(), &records);
        assert_eq!(runs.pages.len(), 2);
        assert!(runs.edges.iter().all(|e| !(e.from == "B" && e.to == "A")));
        let unique = analyze(
            ProductionGraphMode::UniqueRuns,
            &names(),
            &texts(),
            &records,
        );
        assert_eq!(unique.pages.len(), 1);
        assert_eq!(node(&unique, "A").utilities.len(), 0);
    }

    #[test]
    fn utilities_are_collected_per_page() {
        let records = vec![
            rec(PHistTag::Start, 0, None, &[]),
            rec(PHistTag::Conflict, 100, Some("A"), &["A"]),
            rec(PHistTag::Conflict, 200, Some("B"), &["B"]),
            reward(300, &[("A", 1.0), ("B", 3.0)]),
            rec(PHistTag::Stop, 400, None, &[]),
        ];
        let graph = analyze(ProductionGraphMode::Utilities, &names(), &texts(), &records);
        assert_eq!(graph.pages.len(), 1);
        assert_eq!(node(&graph, "A").utilities, vec![(0, 1.0)]);
        assert_eq!(node(&graph, "B").utilities, vec![(0, 3.0)]);
        assert_eq!(graph.utility_min, Some(1.0));
        assert_eq!(graph.utility_max, Some(3.0));
    }

    #[test]
    fn json_round_trip() {
        let records = vec![
            rec(PHistTag::Start, 0, None, &[]),
            rec(PHistTag::Conflict, 100, Some("A"), &["A"]),
            rec(PHistTag::Conflict, 200, Some("B"), &["B"]),
        ];
        let graph = analyze(ProductionGraphMode::All, &names(), &texts(), &records);
        let back = ProductionGraph::from_json(&graph.to_json()).unwrap();
        assert_eq!(back.mode, graph.mode);
        assert_eq!(back.nodes.len(), graph.nodes.len());
        assert_eq!(back.edges.len(), graph.edges.len());
        assert_eq!(back.pages.len(), graph.pages.len());
    }
}
