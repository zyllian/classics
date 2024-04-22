use crate::level::Level;

const NEIGHBORS: &[(isize, isize, isize)] = &[
	(0, 1, 0),
	(0, -1, 0),
	(-1, 0, 0),
	(1, 0, 0),
	(0, 0, -1),
	(0, 0, 1),
];

/// gets a block's direct neighbors which are in the bounds of the level
pub fn neighbors(level: &Level, x: usize, y: usize, z: usize) -> Vec<(usize, usize, usize)> {
	get_many_relative_coords(level, x, y, z, NEIGHBORS.iter().copied())
}

/// gets a blocks direct neighbors (excluding above the block) which are in the bounds of the level
pub fn neighbors_minus_up(
	level: &Level,
	x: usize,
	y: usize,
	z: usize,
) -> Vec<(usize, usize, usize)> {
	get_many_relative_coords(level, x, y, z, NEIGHBORS.iter().skip(1).copied())
}

/// adds relative coordinates to the given ones, returning `None` if the coordinates would be out of bounds for hte level
pub fn get_relative_coords(
	level: &Level,
	x: usize,
	y: usize,
	z: usize,
	rx: isize,
	ry: isize,
	rz: isize,
) -> Option<(usize, usize, usize)> {
	Some((
		x.checked_add_signed(rx)
			.and_then(|x| (x < level.x_size).then_some(x))?,
		y.checked_add_signed(ry)
			.and_then(|y| (y < level.y_size).then_some(y))?,
		z.checked_add_signed(rz)
			.and_then(|z| (z < level.z_size).then_some(z))?,
	))
}

/// takes an iterator of relative changes to apply to the given coordinates and returns a `Vec` of the ones in bounds of the level
pub fn get_many_relative_coords<T>(
	level: &Level,
	x: usize,
	y: usize,
	z: usize,
	coords: T,
) -> Vec<(usize, usize, usize)>
where
	T: IntoIterator<Item = (isize, isize, isize)>,
{
	coords
		.into_iter()
		.filter_map(|(rx, ry, rz)| get_relative_coords(level, x, y, z, rx, ry, rz))
		.collect()
}
