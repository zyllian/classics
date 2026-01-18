use crate::level::Level;

const NEIGHBORS: &[(isize, isize, isize)] = &[
	(0, 1, 0),
	(0, -1, 0),
	(-1, 0, 0),
	(1, 0, 0),
	(0, 0, -1),
	(0, 0, 1),
];

// /// gets a block's direct neighbors which are in the bounds of the level
// pub fn neighbors(level: &Level, x: usize, y: usize, z: usize) -> Vec<(usize, usize, usize)> {
// 	get_many_relative_coords(level, x, y, z, NEIGHBORS.iter().copied())
// }

/// gets a blocks direct neighbors (excluding above the block) which are in the bounds of the level
pub fn neighbors_minus_up(
	level: &Level,
	x: usize,
	y: usize,
	z: usize,
) -> Vec<(usize, usize, usize)> {
	get_many_relative_coords(level, x, y, z, NEIGHBORS.iter().skip(1).copied())
}

/// gets a block's neighbors (including vertical diagonals) which are in the bounds of the level, i.e. for grass spread
pub fn neighbors_with_vertical_diagonals(
	level: &Level,
	x: usize,
	y: usize,
	z: usize,
) -> Vec<(usize, usize, usize)> {
	let down = NEIGHBORS
		.iter()
		.skip(2)
		.copied()
		.map(|(x, _, z)| (x, -1isize, z));
	let up = NEIGHBORS
		.iter()
		.skip(2)
		.copied()
		.map(|(x, _, z)| (x, 1isize, z));
	get_many_relative_coords(
		level,
		x,
		y,
		z,
		NEIGHBORS.iter().skip(2).copied().chain(down).chain(up),
	)
}

/// gets a block's neighbors, including all diagonals
pub fn neighbors_full(level: &Level, x: usize, y: usize, z: usize) -> Vec<(usize, usize, usize)> {
	let iter = (-1..=1).flat_map(|x| (-1..=1).flat_map(move |y| (-1..=1).map(move |z| (x, y, z))));
	get_many_relative_coords(level, x, y, z, iter)
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
