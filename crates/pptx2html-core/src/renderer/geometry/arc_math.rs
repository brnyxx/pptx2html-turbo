pub(super) fn polar_ellipse_offset(
    radius_x: f64,
    radius_y: f64,
    angle_radians: f64,
) -> Option<(f64, f64)> {
    if !radius_x.is_finite()
        || !radius_y.is_finite()
        || !angle_radians.is_finite()
        || radius_x <= 0.0
        || radius_y <= 0.0
    {
        return None;
    }
    let cosine = angle_radians.cos();
    let sine = angle_radians.sin();
    let denominator = (radius_y * cosine).hypot(radius_x * sine);
    if !denominator.is_finite() || denominator <= f64::EPSILON {
        return None;
    }
    let distance = radius_x * radius_y / denominator;
    let offset = (distance * cosine, distance * sine);
    (offset.0.is_finite() && offset.1.is_finite()).then_some(offset)
}
