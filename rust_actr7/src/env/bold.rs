//! BOLD 2D and 3D brain views, ported from the Tcl Environment dialogs.
//!
//! The 2D view overlays the Tcl `ref-brain.gif` image with fixed module
//! regions. The 3D view renders the Tcl voxel mesh with a small perspective
//! camera and painter's-algorithm hidden-surface removal.

use eframe::egui;

use crate::json::Json;

use super::bold_data::{
    BOLD_2D_CANVAS, BOLD_2D_IMAGE_POS, BOLD_2D_IMAGE_SIZE, BOLD_2D_LABELS, BOLD_2D_REGIONS,
    BOLD_3D_CANVAS, BOLD_3D_FACES, BOLD_3D_SCREEN, BOLD_3D_VERTICES, MODULES,
};

const TWO_PI: f32 = std::f32::consts::PI * 2.0;
const RHO: f32 = 5.0;
const DEFAULT_DIST: f32 = 1200.0;

pub(crate) const COLORS_3D: [egui::Color32; 8] = [
    egui::Color32::from_rgb(255, 0, 0),
    egui::Color32::from_rgb(0, 153, 0),
    egui::Color32::from_rgb(0, 0, 255),
    egui::Color32::from_rgb(170, 170, 0),
    egui::Color32::from_rgb(0, 170, 170),
    egui::Color32::from_rgb(170, 0, 170),
    egui::Color32::from_rgb(0, 238, 0),
    egui::Color32::from_rgb(51, 102, 170),
];

pub(crate) const COLORS_2D: [egui::Color32; 8] = COLORS_3D;

pub(crate) struct BoldRow {
    pub(crate) name: String,
    pub(crate) values: Vec<Option<f32>>,
    pub(crate) module: Option<usize>,
}

pub(crate) struct BoldData {
    pub(crate) times: Vec<f64>,
    pub(crate) rows: Vec<BoldRow>,
}

impl BoldData {
    pub(crate) fn parse(records: &[Json]) -> Self {
        let mut times = Vec::new();
        let mut rows = Vec::new();
        for record in records {
            let Json::Arr(fields) = record else {
                continue;
            };
            let name = fields.first().and_then(|x| x.as_str()).unwrap_or("");
            let values: Vec<Option<f32>> = fields
                .iter()
                .skip(1)
                .map(|v| v.as_f64().map(|n| n as f32))
                .collect();
            if name.eq_ignore_ascii_case("time") {
                times = fields.iter().skip(1).filter_map(|v| v.as_f64()).collect();
            } else {
                rows.push(BoldRow {
                    name: name.to_string(),
                    values,
                    module: module_slot(name),
                });
            }
        }
        let max_len = rows
            .iter()
            .map(|r| r.values.len())
            .max()
            .unwrap_or(0)
            .max(times.len());
        if times.is_empty() {
            times = (0..max_len).map(|i| i as f64).collect();
        }
        BoldData { times, rows }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub(crate) fn max_scan(&self) -> usize {
        let n = self
            .times
            .len()
            .max(self.rows.iter().map(|r| r.values.len()).max().unwrap_or(0));
        n.saturating_sub(1)
    }

    pub(crate) fn value(&self, module: usize, scan: usize) -> Option<f32> {
        self.rows
            .iter()
            .rev()
            .find(|r| r.module == Some(module))
            .and_then(|r| r.values.get(scan).copied().flatten())
    }

    pub(crate) fn has_module(&self, module: usize) -> bool {
        self.rows.iter().any(|r| r.module == Some(module))
    }
}

fn module_slot(name: &str) -> Option<usize> {
    match name.to_ascii_lowercase().as_str() {
        "manual" => Some(0),
        "goal" => Some(1),
        "vocal" => Some(2),
        "imaginal" => Some(3),
        "retrieval" => Some(4),
        "procedural" | "production" => Some(5),
        "aural" => Some(6),
        "visual" => Some(7),
        _ => None,
    }
}

pub(crate) fn decode_brain_image() -> Result<egui::ColorImage, String> {
    const BYTES: &[u8] = include_bytes!("../data/ref-brain.gif");
    let image = image::load_from_memory_with_format(BYTES, image::ImageFormat::Gif)
        .map_err(|e| format!("could not decode ref-brain.gif: {e}"))?;
    let rgba = image.to_rgba8();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        rgba.as_raw(),
    ))
}

fn module_tint(module: usize, level: f32) -> egui::Color32 {
    let color = COLORS_2D[module];
    let level = level.clamp(0.0, 1.0);
    let scale = |c: u8| (c as f32 * level).round() as u8;
    egui::Color32::from_rgb(scale(color.r()), scale(color.g()), scale(color.b()))
}

/// Keep the module hue for the 2D labels while lifting the dark channels so
/// they stay readable against the gray strip left of the brain image.
fn label_tint(module: usize) -> egui::Color32 {
    let color = COLORS_2D[module];
    let lift = |c: u8| (c as u16 * 3 / 4 + 64) as u8;
    egui::Color32::from_rgb(lift(color.r()), lift(color.g()), lift(color.b()))
}

fn gray(value: Option<f32>) -> egui::Color32 {
    match value {
        Some(value) => egui::Color32::from_gray((value.clamp(0.0, 1.0) * 255.0).round() as u8),
        None => egui::Color32::BLACK,
    }
}

fn centered_canvas(outer: egui::Rect, canvas: egui::Vec2) -> egui::Rect {
    let offset = egui::vec2(
        ((outer.width() - canvas.x) * 0.5).max(0.0),
        ((outer.height() - canvas.y) * 0.5).max(0.0),
    );
    egui::Rect::from_min_size(outer.min + offset, canvas)
}

pub(crate) fn draw_2d(
    ui: &mut egui::Ui,
    data: &BoldData,
    scan: usize,
    texture: Option<&egui::TextureHandle>,
    image_error: Option<&str>,
) {
    let canvas = egui::vec2(BOLD_2D_CANVAS[0], BOLD_2D_CANVAS[1]);
    let available = ui.available_size();
    let outer_size = egui::vec2(available.x.max(canvas.x), available.y.max(canvas.y));
    let (outer, _) = ui.allocate_exact_size(outer_size, egui::Sense::hover());
    let rect = centered_canvas(outer, canvas);
    ui.painter_at(outer)
        .rect_filled(outer, 0.0, egui::Color32::BLACK);
    let painter = ui.painter_at(rect);

    if let Some(texture) = texture {
        let image_rect = egui::Rect::from_min_size(
            rect.min + egui::vec2(BOLD_2D_IMAGE_POS[0], BOLD_2D_IMAGE_POS[1]),
            egui::vec2(BOLD_2D_IMAGE_SIZE[0], BOLD_2D_IMAGE_SIZE[1]),
        );
        painter.image(
            texture.id(),
            image_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        // The source GIF has white tile grid lines on every tile's top/left
        // edge, but no right/bottom border on the last column and row. Draw
        // those missing edges just inside the canvas so the montage is framed.
        let border = egui::Stroke::new(1.0, egui::Color32::WHITE);
        let right = image_rect.max.x - 0.5;
        let bottom = image_rect.max.y - 0.5;
        painter.line_segment(
            [
                egui::pos2(right, image_rect.min.y),
                egui::pos2(right, bottom),
            ],
            border,
        );
        painter.line_segment(
            [
                egui::pos2(image_rect.min.x, bottom),
                egui::pos2(right, bottom),
            ],
            border,
        );
    } else if let Some(error) = image_error {
        painter.text(
            rect.min + egui::vec2(12.0, 12.0),
            egui::Align2::LEFT_TOP,
            error,
            egui::FontId::monospace(11.0),
            egui::Color32::RED,
        );
    }

    for &(module, [x1, y1, x2, y2]) in BOLD_2D_REGIONS {
        if !data.has_module(module as usize) {
            continue;
        }
        let image_origin = egui::vec2(BOLD_2D_IMAGE_POS[0], BOLD_2D_IMAGE_POS[1]);
        let region = egui::Rect::from_min_max(
            rect.min + image_origin + egui::vec2(x1, y1),
            rect.min + image_origin + egui::vec2(x2, y2),
        );
        let outline = module_tint(module as usize, 0.75);
        let fill = if scan == 0 {
            egui::Color32::TRANSPARENT
        } else {
            module_tint(
                module as usize,
                data.value(module as usize, scan).unwrap_or(0.0),
            )
        };
        painter.rect(
            region,
            0.0,
            fill,
            egui::Stroke::new(1.0, outline),
            egui::StrokeKind::Inside,
        );
    }

    for &(module, [x, y]) in BOLD_2D_LABELS {
        let color = if data.has_module(module as usize) {
            label_tint(module as usize)
        } else {
            egui::Color32::GRAY
        };
        painter.text(
            rect.min + egui::vec2(x, y),
            egui::Align2::CENTER_CENTER,
            MODULES[module as usize],
            egui::FontId::monospace(10.0),
            color,
        );
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Camera {
    pub(crate) theta: f32,
    pub(crate) phi: f32,
    pub(crate) dist: f32,
    pub(crate) pan: egui::Vec2,
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            theta: 0.0,
            phi: 0.0,
            dist: DEFAULT_DIST,
            pan: egui::Vec2::ZERO,
        }
    }
}

fn wrap_angle(angle: f32) -> f32 {
    if angle > std::f32::consts::PI {
        angle - TWO_PI
    } else if angle < -std::f32::consts::PI {
        angle + TWO_PI
    } else {
        angle
    }
}

fn viewing_transform(theta: f32, phi: f32) -> [[f32; 4]; 4] {
    let sinth = theta.sin();
    let costh = theta.cos();
    let sinph = phi.sin();
    let cosph = phi.cos();
    [
        [-sinth, -costh * cosph, -costh * sinph, 0.0],
        [costh, -sinth * cosph, -sinth * sinph, 0.0],
        [0.0, sinph, -cosph, 0.0],
        [0.0, 0.0, RHO, 1.0],
    ]
}

fn transform_point(m: &[[f32; 4]; 4], point: [f32; 3]) -> [f32; 4] {
    let mut out = [0.0; 4];
    for (k, slot) in out.iter_mut().enumerate() {
        *slot = m[0][k] * point[0] + m[1][k] * point[1] + m[2][k] * point[2] + m[3][k];
    }
    out
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn signed_area(points: &[egui::Pos2]) -> f32 {
    let mut area = 0.0;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        area += a.x * b.y - b.x * a.y;
    }
    area * 0.5
}

fn surface_shade(normal: [f32; 3]) -> egui::Color32 {
    let light = [0.35, 0.55, 0.76];
    let diffuse = dot(normal, light).abs().clamp(0.0, 1.0);
    let value = 0.15 + 0.45 * diffuse;
    egui::Color32::from_gray((value * 255.0).round() as u8)
}

#[derive(Clone, Copy)]
struct RasterVertex {
    pos: egui::Pos2,
    inv_z: f32,
    color: egui::Color32,
}

fn project_vertex(m: &[[f32; 4]; 4], camera: &Camera, vertex: [f32; 3]) -> Option<RasterVertex> {
    let transformed = transform_point(m, vertex);
    let z = transformed[2];
    if z <= 0.1 {
        return None;
    }
    Some(RasterVertex {
        pos: egui::pos2(
            BOLD_3D_SCREEN[0] / 2.0 + camera.dist * transformed[0] / z + camera.pan.x,
            BOLD_3D_SCREEN[1] / 2.0 - camera.dist * transformed[1] / z - camera.pan.y,
        ),
        inv_z: 1.0 / z,
        color: egui::Color32::BLACK,
    })
}

fn edge(a: egui::Pos2, b: egui::Pos2, p: egui::Pos2) -> f32 {
    (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)
}

fn raster_triangle(
    pixels: &mut [egui::Color32],
    depth: &mut [f32],
    width: usize,
    height: usize,
    a: RasterVertex,
    b: RasterVertex,
    c: RasterVertex,
) {
    let min_x = a.pos.x.min(b.pos.x).min(c.pos.x);
    let max_x = a.pos.x.max(b.pos.x).max(c.pos.x);
    let min_y = a.pos.y.min(b.pos.y).min(c.pos.y);
    let max_y = a.pos.y.max(b.pos.y).max(c.pos.y);
    if max_x < 0.0 || max_y < 0.0 || min_x >= width as f32 || min_y >= height as f32 {
        return;
    }
    let x0 = min_x.floor().max(0.0) as usize;
    let x1 = max_x.ceil().min((width - 1) as f32) as usize;
    let y0 = min_y.floor().max(0.0) as usize;
    let y1 = max_y.ceil().min((height - 1) as f32) as usize;
    let area = edge(a.pos, b.pos, c.pos);
    if area.abs() < 1e-6 {
        return;
    }

    for y in y0..=y1 {
        let py = y as f32 + 0.5;
        for x in x0..=x1 {
            let p = egui::pos2(x as f32 + 0.5, py);
            let w0 = edge(b.pos, c.pos, p) / area;
            let w1 = edge(c.pos, a.pos, p) / area;
            let w2 = edge(a.pos, b.pos, p) / area;
            if w0 < -1e-4 || w1 < -1e-4 || w2 < -1e-4 {
                continue;
            }
            let index = y * width + x;
            let inv_z = w0 * a.inv_z + w1 * b.inv_z + w2 * c.inv_z;
            if inv_z <= depth[index] {
                continue;
            }
            depth[index] = inv_z;
            let mix = |ca: u8, cb: u8, cc: u8| {
                (w0 * ca as f32 + w1 * cb as f32 + w2 * cc as f32)
                    .round()
                    .clamp(0.0, 255.0) as u8
            };
            pixels[index] = egui::Color32::from_rgb(
                mix(a.color.r(), b.color.r(), c.color.r()),
                mix(a.color.g(), b.color.g(), c.color.g()),
                mix(a.color.b(), b.color.b(), c.color.b()),
            );
        }
    }
}

fn plot_line_pixel(
    pixels: &mut [egui::Color32],
    depth: &mut [f32],
    width: usize,
    height: usize,
    x: isize,
    y: isize,
    inv_z: f32,
    color: egui::Color32,
) {
    if x < 0 || y < 0 || x >= width as isize || y >= height as isize {
        return;
    }
    let index = y as usize * width + x as usize;
    if inv_z + 0.002 < depth[index] {
        return;
    }
    pixels[index] = color;
    if inv_z > depth[index] {
        depth[index] = inv_z;
    }
}

fn draw_line(
    pixels: &mut [egui::Color32],
    depth: &mut [f32],
    width: usize,
    height: usize,
    a: RasterVertex,
    b: RasterVertex,
    color: egui::Color32,
) {
    let steps = (b.pos.x - a.pos.x)
        .abs()
        .max((b.pos.y - a.pos.y).abs())
        .ceil() as usize;
    let steps = steps.max(1);
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let x = (a.pos.x + (b.pos.x - a.pos.x) * t).round() as isize;
        let y = (a.pos.y + (b.pos.y - a.pos.y) * t).round() as isize;
        let inv_z = a.inv_z + (b.inv_z - a.inv_z) * t;
        for (ox, oy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            plot_line_pixel(pixels, depth, width, height, x + ox, y + oy, inv_z, color);
        }
    }
}

fn convex_hull(mut points: Vec<egui::Pos2>) -> Vec<egui::Pos2> {
    points.sort_by(|a, b| a.x.total_cmp(&b.x).then_with(|| a.y.total_cmp(&b.y)));
    points.dedup_by(|a, b| a.x == b.x && a.y == b.y);
    if points.len() <= 2 {
        return points;
    }
    let cross = |o: egui::Pos2, a: egui::Pos2, b: egui::Pos2| {
        (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
    };
    let mut lower = Vec::new();
    for &point in &points {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], point) <= 0.0
        {
            lower.pop();
        }
        lower.push(point);
    }
    let mut upper = Vec::new();
    for &point in points.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], point) <= 0.0
        {
            upper.pop();
        }
        upper.push(point);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn render_3d_image(data: &BoldData, scan: usize, camera: &Camera) -> egui::ColorImage {
    const WIDTH: usize = 650;
    const HEIGHT: usize = 500;
    let mut pixels = vec![egui::Color32::BLACK; WIDTH * HEIGHT];
    let mut depth = vec![f32::NEG_INFINITY; WIDTH * HEIGHT];
    let matrix = viewing_transform(camera.theta, camera.phi);

    // The Tcl shell is a set of surface strips, which leaves visible seams in
    // a solid render. Use its projected silhouette as a smooth-shaded shell
    // instead; the module voxels are drawn over it in a separate pass.
    let mut silhouette = Vec::new();
    for face in BOLD_3D_FACES.iter().filter(|face| face.module.is_none()) {
        for i in 0..face.len as usize {
            if let Some(vertex) = project_vertex(&matrix, camera, BOLD_3D_VERTICES[face.verts[i]]) {
                silhouette.push(vertex.pos);
            }
        }
    }
    let hull = convex_hull(silhouette);
    if hull.len() >= 3 {
        let center = egui::pos2(
            hull.iter().map(|p| p.x).sum::<f32>() / hull.len() as f32,
            hull.iter().map(|p| p.y).sum::<f32>() / hull.len() as f32,
        );
        let radius_x = hull
            .iter()
            .map(|p| (p.x - center.x).abs())
            .fold(1.0f32, f32::max)
            .max(1.0);
        let radius_y = hull
            .iter()
            .map(|p| (p.y - center.y).abs())
            .fold(1.0f32, f32::max)
            .max(1.0);
        let shell_vertex = |point: egui::Pos2| {
            let x = (point.x - center.x) / radius_x;
            let y = (point.y - center.y) / radius_y;
            let z = (1.0 - x * x - y * y).max(0.0).sqrt();
            RasterVertex {
                pos: point,
                inv_z: 0.0,
                color: surface_shade([x, y, z]),
            }
        };
        let shell: Vec<_> = hull.iter().map(|&point| shell_vertex(point)).collect();
        for i in 1..shell.len() - 1 {
            raster_triangle(
                &mut pixels,
                &mut depth,
                WIDTH,
                HEIGHT,
                shell[0],
                shell[i],
                shell[i + 1],
            );
        }
    }

    // Draw the module voxels in a second, separate depth pass so they remain
    // visible through the solid shell.
    depth.fill(0.0);
    for face in BOLD_3D_FACES.iter().filter(|face| face.module.is_some()) {
        let module = face.module.expect("module face");
        let color = gray(data.value(module as usize, scan));
        let mut vertices = Vec::with_capacity(face.len as usize);
        let mut visible = true;
        for i in 0..face.len as usize {
            match project_vertex(&matrix, camera, BOLD_3D_VERTICES[face.verts[i]]) {
                Some(mut vertex) => {
                    vertex.color = color;
                    vertices.push(vertex);
                }
                None => {
                    visible = false;
                    break;
                }
            }
        }
        if !visible {
            continue;
        }
        for i in 1..vertices.len() - 1 {
            raster_triangle(
                &mut pixels,
                &mut depth,
                WIDTH,
                HEIGHT,
                vertices[0],
                vertices[i],
                vertices[i + 1],
            );
        }
    }

    for face in BOLD_3D_FACES.iter().filter(|face| face.module.is_some()) {
        let module = face.module.expect("module face");
        let mut vertices = Vec::with_capacity(face.len as usize);
        let mut visible = true;
        for i in 0..face.len as usize {
            match project_vertex(&matrix, camera, BOLD_3D_VERTICES[face.verts[i]]) {
                Some(vertex) => vertices.push(vertex),
                None => {
                    visible = false;
                    break;
                }
            }
        }
        if !visible {
            continue;
        }
        let points: Vec<_> = vertices.iter().map(|v| v.pos).collect();
        if signed_area(&points) >= -0.01 {
            continue;
        }
        for i in 0..vertices.len() {
            draw_line(
                &mut pixels,
                &mut depth,
                WIDTH,
                HEIGHT,
                vertices[i],
                vertices[(i + 1) % vertices.len()],
                COLORS_3D[module as usize],
            );
        }
    }

    egui::ColorImage::new([WIDTH, HEIGHT], pixels)
}

pub(crate) fn draw_3d(
    ui: &mut egui::Ui,
    data: &BoldData,
    scan: usize,
    camera: &mut Camera,
    texture: &mut Option<egui::TextureHandle>,
) {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(BOLD_3D_CANVAS[0], BOLD_3D_CANVAS[1]),
        egui::Sense::click_and_drag(),
    );
    let delta = ui.input(|i| i.pointer.delta());
    if response.dragged_by(egui::PointerButton::Primary) {
        camera.theta = wrap_angle(camera.theta - delta.x * 0.01);
        camera.phi = wrap_angle(camera.phi - delta.y * 0.01);
    }
    if response.dragged_by(egui::PointerButton::Middle) {
        camera.pan.x += delta.x;
        camera.pan.y -= delta.y;
    }
    if response.dragged_by(egui::PointerButton::Secondary) {
        camera.dist += -delta.y * 2.0;
    }

    let image = render_3d_image(data, scan, camera);
    if let Some(handle) = texture.as_mut() {
        handle.set(image, egui::TextureOptions::LINEAR);
    } else {
        *texture = Some(ui.ctx().load_texture(
            "bold-3d-brain",
            image,
            egui::TextureOptions::LINEAR,
        ));
    }

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, egui::Color32::BLACK);
    if let Some(handle) = texture.as_ref() {
        painter.image(
            handle.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }

    for (module, &name) in MODULES.iter().enumerate() {
        let y = 200.0 + module as f32 * 40.0;
        painter.text(
            rect.min + egui::vec2(10.0, y),
            egui::Align2::LEFT_CENTER,
            name,
            egui::FontId::monospace(10.0),
            COLORS_3D[module],
        );
        let value = data.value(module, scan);
        painter.text(
            rect.min + egui::vec2(70.0, y),
            egui::Align2::LEFT_CENTER,
            value
                .map(|v| format!("{v:.3}"))
                .unwrap_or_else(|| "---".to_string()),
            egui::FontId::monospace(10.0),
            egui::Color32::from_gray(0xcc),
        );
    }

    painter.text(
        rect.min + egui::vec2(10.0, 10.0),
        egui::Align2::LEFT_CENTER,
        "0.0",
        egui::FontId::monospace(9.0),
        egui::Color32::from_gray(0xdd),
    );
    painter.text(
        rect.min + egui::vec2(10.0, 170.0),
        egui::Align2::LEFT_CENTER,
        "1.0",
        egui::FontId::monospace(9.0),
        egui::Color32::from_gray(0xdd),
    );
    for c in (0..256).step_by(3) {
        let y = (c / 3 * 2) as f32;
        painter.rect_filled(
            egui::Rect::from_min_size(rect.min + egui::vec2(30.0, y), egui::vec2(10.0, 2.0)),
            0.0,
            egui::Color32::from_gray(c as u8),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_missing_bold_values_and_module_aliases() {
        let records = vec![
            Json::Arr(vec![
                Json::Str("time".into()),
                Json::Float(0.0),
                Json::Float(2.0),
            ]),
            Json::Arr(vec![Json::Str("goal".into()), Json::Float(0.1), Json::Null]),
            Json::Arr(vec![
                Json::Str("production".into()),
                Json::Null,
                Json::Float(0.7),
            ]),
        ];
        let data = BoldData::parse(&records);
        assert_eq!(data.times, vec![0.0, 2.0]);
        assert_eq!(data.value(1, 0), Some(0.1));
        assert_eq!(data.value(1, 1), None);
        assert_eq!(data.value(5, 1), Some(0.7));
        assert!(!data.has_module(0));
        assert!(data.has_module(5));
    }

    #[test]
    fn brain_image_decodes_at_reference_size() {
        let image = decode_brain_image().expect("ref-brain.gif");
        assert_eq!(image.size, [640, 640]);
    }

    #[test]
    fn projection_matches_tcl_center_and_rotates() {
        let camera = Camera::default();
        let matrix = viewing_transform(camera.theta, camera.phi);
        let center = project_vertex(&matrix, &camera, [0.0, 0.0, 0.0])
            .expect("center")
            .pos;
        assert!((center.x - 375.0).abs() < 0.01);
        assert!((center.y - 250.0).abs() < 0.01);

        let rotated = Camera {
            theta: 0.3,
            ..Camera::default()
        };
        let matrix = viewing_transform(rotated.theta, rotated.phi);
        let moved = project_vertex(&matrix, &rotated, [0.5, 0.0, 0.0])
            .expect("rotated point")
            .pos;
        assert!((moved.x - center.x).abs() > 1.0);
    }

    #[test]
    fn centers_the_brain_canvas_in_available_space() {
        let outer = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(800.0, 700.0));
        let rect = centered_canvas(outer, egui::vec2(640.0, 660.0));
        assert_eq!(rect.min, egui::pos2(90.0, 40.0));
        assert_eq!(rect.size(), egui::vec2(640.0, 660.0));
    }

    #[test]
    fn two_d_uses_the_three_d_module_palette() {
        assert_eq!(COLORS_2D, COLORS_3D);
        assert_ne!(COLORS_2D[4], COLORS_2D[5]);
    }

    #[test]
    fn projected_winding_identifies_backfaces() {
        let front = [
            egui::pos2(0.0, 0.0),
            egui::pos2(0.0, 10.0),
            egui::pos2(10.0, 0.0),
        ];
        assert!(signed_area(&front) < 0.0);
        let back = [front[0], front[2], front[1]];
        assert!(signed_area(&back) > 0.0);
    }

    #[test]
    fn surface_shading_stays_visible() {
        for normal in [[0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            let value = surface_shade(normal).r();
            assert!((38..=153).contains(&value), "shade {value}");
        }
    }

    #[test]
    fn tcl_mesh_and_regions_are_intact() {
        assert_eq!(BOLD_3D_VERTICES.len(), 479);
        assert_eq!(BOLD_3D_FACES.len(), 271);
        assert_eq!(BOLD_2D_REGIONS.len(), 62);
        assert_eq!(BOLD_2D_LABELS.len(), 8);
        for &(_, [x1, y1, x2, y2]) in BOLD_2D_REGIONS {
            assert!((0.0..=BOLD_2D_IMAGE_SIZE[0]).contains(&x1));
            assert!((0.0..=BOLD_2D_IMAGE_SIZE[1]).contains(&y1));
            assert!((0.0..=BOLD_2D_IMAGE_SIZE[0]).contains(&x2));
            assert!((0.0..=BOLD_2D_IMAGE_SIZE[1]).contains(&y2));
        }
        for &(_, [x, y]) in BOLD_2D_LABELS {
            assert!((0.0..=BOLD_2D_IMAGE_SIZE[0]).contains(&x));
            assert!(y < BOLD_2D_IMAGE_POS[1]);
        }
        for face in BOLD_3D_FACES {
            for &index in &face.verts[..face.len as usize] {
                assert!(index < BOLD_3D_VERTICES.len());
            }
        }
        let module_faces = BOLD_3D_FACES.iter().filter(|f| f.module.is_some()).count();
        assert_eq!(module_faces, 96);
    }

    #[test]
    fn software_rasterizer_fills_and_depth_tests() {
        let mut pixels = vec![egui::Color32::BLACK; 16 * 16];
        let mut depth = vec![0.0f32; 16 * 16];
        let far = RasterVertex {
            pos: egui::pos2(0.0, 0.0),
            inv_z: 0.1,
            color: egui::Color32::RED,
        };
        let mut triangle = [
            far,
            RasterVertex {
                pos: egui::pos2(15.0, 0.0),
                inv_z: 0.1,
                color: egui::Color32::RED,
            },
            RasterVertex {
                pos: egui::pos2(0.0, 15.0),
                inv_z: 0.1,
                color: egui::Color32::RED,
            },
        ];
        raster_triangle(
            &mut pixels,
            &mut depth,
            16,
            16,
            triangle[0],
            triangle[1],
            triangle[2],
        );
        assert_eq!(pixels[8 * 16 + 3], egui::Color32::RED);

        triangle[0].inv_z = 0.2;
        triangle[1].inv_z = 0.2;
        triangle[2].inv_z = 0.2;
        triangle[0].color = egui::Color32::BLUE;
        triangle[1].color = egui::Color32::BLUE;
        triangle[2].color = egui::Color32::BLUE;
        raster_triangle(
            &mut pixels,
            &mut depth,
            16,
            16,
            triangle[0],
            triangle[1],
            triangle[2],
        );
        assert_eq!(pixels[8 * 16 + 3], egui::Color32::BLUE);
    }

    #[test]
    fn software_renderer_produces_the_3d_canvas() {
        let data = BoldData::parse(&[]);
        let image = render_3d_image(&data, 0, &Camera::default());
        assert_eq!(image.size, [650, 500]);
        assert!(image
            .pixels
            .iter()
            .any(|pixel| *pixel != egui::Color32::BLACK));
    }
}
