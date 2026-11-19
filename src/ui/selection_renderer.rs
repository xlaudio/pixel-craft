//! Static, high-contrast rendering for all selection region outlines.

use eframe::egui;
use egui::{Color32, Painter, Pos2, Stroke};

use crate::model::SelectionRegion;

const ELLIPSE_SEGMENTS: usize = 96;
const DASH_LENGTH: f32 = 6.0;
const GAP_LENGTH: f32 = 3.0;
const STROKE_WIDTH: f32 = 2.0;

pub fn render(painter: &Painter, region: &SelectionRegion, to_screen: impl Fn(Pos2) -> Pos2) {
    let points = contour_points(region);
    if points.len() < 2 {
        return;
    }

    let points: Vec<_> = points.into_iter().map(to_screen).collect();
    for (start, end, color) in dashed_segments(&points) {
        painter.line_segment([start, end], Stroke::new(STROKE_WIDTH, color));
    }
}

fn dashed_segments(points: &[Pos2]) -> Vec<(Pos2, Pos2, Color32)> {
    if points.len() < 2 {
        return Vec::new();
    }

    let mut distance_along_contour = 0.0;
    let pattern_length = DASH_LENGTH + GAP_LENGTH;
    let mut segments = Vec::new();

    for (start, end) in points
        .iter()
        .copied()
        .zip(points.iter().copied().cycle().skip(1))
        .take(points.len())
    {
        let edge = end - start;
        let edge_length = edge.length();
        if edge_length <= f32::EPSILON {
            continue;
        }

        let mut edge_distance = 0.0;
        while edge_distance < edge_length {
            let pattern_position =
                (distance_along_contour + edge_distance).rem_euclid(pattern_length);
            let dash_index =
                ((distance_along_contour + edge_distance) / pattern_length).floor() as usize;
            if pattern_position < DASH_LENGTH {
                let dash_end = (edge_distance + DASH_LENGTH - pattern_position).min(edge_length);
                let color = if dash_index % 2 == 0 {
                    Color32::BLACK
                } else {
                    Color32::WHITE
                };
                segments.push((
                    start + edge * (edge_distance / edge_length),
                    start + edge * (dash_end / edge_length),
                    color,
                ));
                edge_distance = dash_end;
            } else {
                edge_distance =
                    (edge_distance + pattern_length - pattern_position).min(edge_length);
            }
        }
        distance_along_contour += edge_length;
    }
    segments
}

fn contour_points(region: &SelectionRegion) -> Vec<Pos2> {
    match region {
        SelectionRegion::Rectangle(rect) => {
            let rect = egui::Rect::from_two_pos(rect.min, rect.max);
            vec![
                rect.left_top(),
                rect.right_top(),
                rect.right_bottom(),
                rect.left_bottom(),
            ]
        }
        SelectionRegion::Ellipse(rect) => {
            let rect = egui::Rect::from_two_pos(rect.min, rect.max);
            (0..ELLIPSE_SEGMENTS)
                .map(|step| {
                    let angle = step as f32 * (std::f32::consts::TAU / ELLIPSE_SEGMENTS as f32);
                    Pos2::new(
                        rect.center().x + angle.cos() * rect.width() / 2.0,
                        rect.center().y + angle.sin() * rect.height() / 2.0,
                    )
                })
                .collect()
        }
        SelectionRegion::Polygon(points) => {
            let mut contour = points.clone();
            if contour.first() == contour.last() {
                contour.pop();
            }
            contour
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{contour_points, dashed_segments, DASH_LENGTH};
    use crate::model::SelectionRegion;
    use egui::{pos2, Rect};

    #[test]
    fn renderer_builds_closed_contours_for_each_selection_geometry() {
        let rectangle = contour_points(&SelectionRegion::Rectangle(Rect::from_min_max(
            pos2(0.0, 0.0),
            pos2(10.0, 8.0),
        )));
        assert_eq!(rectangle.len(), 4);
        assert_eq!(rectangle[0], pos2(0.0, 0.0));

        let ellipse = contour_points(&SelectionRegion::Ellipse(Rect::from_min_max(
            pos2(0.0, 0.0),
            pos2(10.0, 8.0),
        )));
        assert_eq!(ellipse.len(), 96);

        let polygon = contour_points(&SelectionRegion::Polygon(vec![
            pos2(1.0, 1.0),
            pos2(8.0, 2.0),
            pos2(5.0, 9.0),
            pos2(1.0, 1.0),
        ]));
        assert_eq!(polygon.len(), 3);
    }

    #[test]
    fn renderer_emits_static_alternating_dashes() {
        let rectangle = contour_points(&SelectionRegion::Rectangle(Rect::from_min_max(
            pos2(0.0, 0.0),
            pos2(30.0, 20.0),
        )));
        let dashes = dashed_segments(&rectangle);

        assert!(dashes.len() > 2);
        assert_eq!(dashes[0].2, egui::Color32::BLACK);
        assert_eq!(dashes[1].2, egui::Color32::WHITE);
        assert!(dashes.iter().all(|(start, end, _)| {
            ((end.x - start.x).powi(2) + (end.y - start.y).powi(2)).sqrt()
                <= DASH_LENGTH + f32::EPSILON
        }));
    }
}
