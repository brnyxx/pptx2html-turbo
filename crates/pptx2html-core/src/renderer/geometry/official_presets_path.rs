use std::fmt::Write;

use super::arc_math::polar_ellipse_offset;
use super::official_presets_formula::GuideEnvironment;
use super::shared::CustomGeomPathSvg;
use crate::model::PathFill;

#[derive(Debug)]
pub(super) struct PathDefinition {
    pub(super) width: Option<String>,
    pub(super) height: Option<String>,
    pub(super) fill: PathFill,
    pub(super) stroke: bool,
    pub(super) commands: Vec<PathCommandDefinition>,
}

#[derive(Debug)]
pub(super) enum PathCommandDefinition {
    Move(Vec<PointDefinition>),
    Line(Vec<PointDefinition>),
    Cubic(Vec<PointDefinition>),
    Quad(Vec<PointDefinition>),
    Arc {
        width_radius: String,
        height_radius: String,
        start_angle: String,
        swing_angle: String,
    },
    Close,
}

#[derive(Debug)]
pub(super) struct PointDefinition {
    pub(super) x: String,
    pub(super) y: String,
}

pub(super) fn render_path(
    definition: &PathDefinition,
    environment: &GuideEnvironment,
    shape_width: f64,
    shape_height: f64,
) -> Result<CustomGeomPathSvg, String> {
    let path_width = definition
        .width
        .as_deref()
        .map(|value| environment.resolve(value))
        .transpose()?
        .unwrap_or(shape_width);
    let path_height = definition
        .height
        .as_deref()
        .map(|value| environment.resolve(value))
        .transpose()?
        .unwrap_or(shape_height);
    let scale_x = safe_scale(shape_width, path_width);
    let scale_y = safe_scale(shape_height, path_height);
    let mut output = String::with_capacity(256);
    let mut current = (0.0, 0.0);
    for command in &definition.commands {
        match command {
            PathCommandDefinition::Move(points) => {
                let point = one_point(points, "moveTo")?;
                current = resolve_point(point, environment, scale_x, scale_y)?;
                let _ = write!(output, "M{:.2},{:.2} ", current.0, current.1);
            }
            PathCommandDefinition::Line(points) => {
                let point = one_point(points, "lnTo")?;
                current = resolve_point(point, environment, scale_x, scale_y)?;
                let _ = write!(output, "L{:.2},{:.2} ", current.0, current.1);
            }
            PathCommandDefinition::Cubic(points) => {
                if points.len() != 3 {
                    return Err("cubicBezTo requires three points".into());
                }
                let first = resolve_point(&points[0], environment, scale_x, scale_y)?;
                let second = resolve_point(&points[1], environment, scale_x, scale_y)?;
                current = resolve_point(&points[2], environment, scale_x, scale_y)?;
                let _ = write!(
                    output,
                    "C{:.2},{:.2} {:.2},{:.2} {:.2},{:.2} ",
                    first.0, first.1, second.0, second.1, current.0, current.1
                );
            }
            PathCommandDefinition::Quad(points) => {
                if points.len() != 2 {
                    return Err("quadBezTo requires two points".into());
                }
                let control = resolve_point(&points[0], environment, scale_x, scale_y)?;
                current = resolve_point(&points[1], environment, scale_x, scale_y)?;
                let _ = write!(
                    output,
                    "Q{:.2},{:.2} {:.2},{:.2} ",
                    control.0, control.1, current.0, current.1
                );
            }
            PathCommandDefinition::Arc {
                width_radius,
                height_radius,
                start_angle,
                swing_angle,
            } => {
                let radius_x = environment.resolve(width_radius)?;
                let radius_y = environment.resolve(height_radius)?;
                let start = environment.resolve(start_angle)?;
                let swing = environment.resolve(swing_angle)?;
                if swing.abs() < 0.001 {
                    continue;
                }
                if radius_x.abs() < 0.001 || radius_y.abs() < 0.001 {
                    let start_radians = ooxml_radians(start);
                    let end_radians = ooxml_radians(start + swing);
                    let delta_x = if radius_y.abs() < 0.001 {
                        radius_x * (end_radians.cos() - start_radians.cos()) * scale_x
                    } else {
                        0.0
                    };
                    let delta_y = if radius_x.abs() < 0.001 {
                        radius_y * (end_radians.sin() - start_radians.sin()) * scale_y
                    } else {
                        0.0
                    };
                    current.0 += delta_x;
                    current.1 += delta_y;
                    if delta_x.abs() >= 0.001 || delta_y.abs() >= 0.001 {
                        let _ = write!(output, "L{:.2},{:.2} ", current.0, current.1);
                    }
                    continue;
                }
                append_arc(
                    &mut output,
                    &mut current,
                    (radius_x, radius_y),
                    (scale_x, scale_y),
                    start,
                    swing,
                )?;
            }
            PathCommandDefinition::Close => output.push_str("Z "),
        }
    }
    Ok(CustomGeomPathSvg {
        d: output.trim_end().to_owned(),
        fill: definition.fill.clone(),
        stroke: definition.stroke,
    })
}

fn append_arc(
    output: &mut String,
    current: &mut (f64, f64),
    radii: (f64, f64),
    scale: (f64, f64),
    start: f64,
    swing: f64,
) -> Result<(), String> {
    const HALF_TURN: f64 = 10_800_000.0;
    const MAX_SEGMENTS: f64 = 2_048.0;
    let segment_count = (swing.abs() / HALF_TURN).ceil().max(1.0);
    if !segment_count.is_finite() || segment_count > MAX_SEGMENTS {
        return Err(format!("unrepresentable arc sweep: {swing}"));
    }
    let segments = segment_count as usize;
    let segment_swing = swing / segments as f64;
    let origin = *current;
    let (radius_x, radius_y) = radii;
    let (scale_x, scale_y) = scale;
    let start_radians = ooxml_radians(start);
    let start_offset = polar_ellipse_offset(radius_x, radius_y, start_radians)
        .ok_or_else(|| format!("unrepresentable arc radii: {radius_x}x{radius_y}"))?;
    let scaled_radius_x = radius_x * scale_x;
    let scaled_radius_y = radius_y * scale_y;
    let sweep_flag = i32::from(swing > 0.0);
    for index in 1..=segments {
        let end_radians = ooxml_radians(start + segment_swing * index as f64);
        let end_offset = polar_ellipse_offset(radius_x, radius_y, end_radians)
            .ok_or_else(|| format!("unrepresentable arc angle: {end_radians}"))?;
        current.0 = origin.0 + (end_offset.0 - start_offset.0) * scale_x;
        current.1 = origin.1 + (end_offset.1 - start_offset.1) * scale_y;
        let _ = write!(
            output,
            "A{scaled_radius_x:.2},{scaled_radius_y:.2} 0 0,{sweep_flag} {:.2},{:.2} ",
            current.0, current.1
        );
    }
    Ok(())
}

fn one_point<'a>(
    points: &'a [PointDefinition],
    command: &str,
) -> Result<&'a PointDefinition, String> {
    if points.len() == 1 {
        Ok(&points[0])
    } else {
        Err(format!("{command} requires one point"))
    }
}

fn resolve_point(
    point: &PointDefinition,
    environment: &GuideEnvironment,
    scale_x: f64,
    scale_y: f64,
) -> Result<(f64, f64), String> {
    Ok((
        environment.resolve(&point.x)? * scale_x,
        environment.resolve(&point.y)? * scale_y,
    ))
}

fn safe_scale(shape: f64, path: f64) -> f64 {
    if shape.is_finite() && path.is_finite() && path.abs() >= f64::EPSILON {
        shape.max(0.0) / path
    } else {
        0.0
    }
}

fn ooxml_radians(angle: f64) -> f64 {
    (angle / 60_000.0).to_radians()
}

#[cfg(test)]
mod tests {
    use super::{PathCommandDefinition, PathDefinition, PointDefinition, render_path};
    use crate::model::PathFill;
    use crate::renderer::geometry::official_presets_formula::GuideEnvironment;

    #[test]
    fn every_official_path_command_emits_svg_topology() {
        let definition = PathDefinition {
            width: None,
            height: None,
            fill: PathFill::Norm,
            stroke: true,
            commands: vec![
                PathCommandDefinition::Move(vec![point("0", "0")]),
                PathCommandDefinition::Line(vec![point("10", "0")]),
                PathCommandDefinition::Quad(vec![point("15", "5"), point("10", "10")]),
                PathCommandDefinition::Cubic(vec![
                    point("5", "15"),
                    point("0", "15"),
                    point("0", "10"),
                ]),
                PathCommandDefinition::Arc {
                    width_radius: "5".into(),
                    height_radius: "5".into(),
                    start_angle: "0".into(),
                    swing_angle: "5400000".into(),
                },
                PathCommandDefinition::Close,
            ],
        };
        let path = render_path(&definition, &GuideEnvironment::new(20.0, 20.0), 20.0, 20.0)
            .expect("official path");
        assert_eq!(
            path.d
                .chars()
                .filter(char::is_ascii_alphabetic)
                .collect::<String>(),
            "MLQCAZ"
        );
    }

    #[test]
    fn unequal_radii_arc_uses_ooxml_ray_angle_endpoint() {
        let definition = PathDefinition {
            width: Some("40".into()),
            height: Some("20".into()),
            fill: PathFill::Norm,
            stroke: true,
            commands: vec![
                PathCommandDefinition::Move(vec![point("0", "10")]),
                PathCommandDefinition::Arc {
                    width_radius: "20".into(),
                    height_radius: "10".into(),
                    start_angle: "10800000".into(),
                    swing_angle: "-2700000".into(),
                },
            ],
        };

        let path = render_path(&definition, &GuideEnvironment::new(40.0, 20.0), 40.0, 20.0)
            .expect("unequal-radii official arc");

        assert_eq!(path.d, "M0.00,10.00 A20.00,10.00 0 0,0 11.06,18.94");
    }

    #[test]
    fn zero_height_arc_degenerates_to_its_horizontal_chord() {
        let definition = PathDefinition {
            width: Some("40".into()),
            height: Some("20".into()),
            fill: PathFill::Norm,
            stroke: true,
            commands: vec![
                PathCommandDefinition::Move(vec![point("0", "0")]),
                PathCommandDefinition::Arc {
                    width_radius: "20".into(),
                    height_radius: "0".into(),
                    start_angle: "10800000".into(),
                    swing_angle: "-10800000".into(),
                },
            ],
        };

        let path = render_path(&definition, &GuideEnvironment::new(40.0, 20.0), 40.0, 20.0)
            .expect("zero-height official arc");

        assert_eq!(path.d, "M0.00,0.00 L40.00,0.00");
    }

    fn point(x: &str, y: &str) -> PointDefinition {
        PointDefinition {
            x: x.into(),
            y: y.into(),
        }
    }
}
