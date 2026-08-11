use std::fmt::Write;

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
                let radius_x = environment.resolve(width_radius)? * scale_x;
                let radius_y = environment.resolve(height_radius)? * scale_y;
                let start = environment.resolve(start_angle)?;
                let swing = environment.resolve(swing_angle)?;
                if radius_x.abs() < 0.001 || radius_y.abs() < 0.001 || swing.abs() < 0.001 {
                    continue;
                }
                append_arc(&mut output, &mut current, radius_x, radius_y, start, swing)?;
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
    radius_x: f64,
    radius_y: f64,
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
    let start_radians = ooxml_radians(start);
    let sweep_flag = i32::from(swing > 0.0);
    for index in 1..=segments {
        let end_radians = ooxml_radians(start + segment_swing * index as f64);
        current.0 = origin.0 + radius_x * (end_radians.cos() - start_radians.cos());
        current.1 = origin.1 + radius_y * (end_radians.sin() - start_radians.sin());
        let _ = write!(
            output,
            "A{radius_x:.2},{radius_y:.2} 0 0,{sweep_flag} {:.2},{:.2} ",
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

    fn point(x: &str, y: &str) -> PointDefinition {
        PointDefinition {
            x: x.into(),
            y: y.into(),
        }
    }
}
