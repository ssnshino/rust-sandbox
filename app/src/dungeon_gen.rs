use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::Rng;
use rand::SeedableRng;

pub const COLS: usize = 41;
pub const ROWS: usize = 41;

fn floor_size(floor: u32) -> (usize, usize) {
    match floor {
        1 => (17, 17),
        2 => (23, 23),
        3 => (29, 29),
        _ => (COLS, ROWS),
    }
}

/// Recursive-backtracker maze + braid (extra loops for escape routes)
pub fn generate_for_floor(seed: u64, floor: u32) -> Vec<Vec<u8>> {
    let mut grid = vec![vec![0u8; COLS]; ROWS];
    let mut rng = SmallRng::seed_from_u64(seed);
    let (active_cols, active_rows) = floor_size(floor);
    carve(&mut grid, 1, 1, active_cols, active_rows, &mut rng);
    add_braids(&mut grid, active_cols, active_rows, &mut rng, 0.18);
    grid
}

fn carve(
    grid: &mut Vec<Vec<u8>>,
    x: usize,
    y: usize,
    active_cols: usize,
    active_rows: usize,
    rng: &mut SmallRng,
) {
    grid[y][x] = 1;
    let mut dirs: [(i32, i32); 4] = [(0, -2), (0, 2), (-2, 0), (2, 0)];
    dirs.shuffle(rng);
    for (dx, dy) in dirs {
        let nx = x as i32 + dx;
        let ny = y as i32 + dy;
        if nx > 0 && ny > 0 && (nx as usize) < active_cols - 1 && (ny as usize) < active_rows - 1 {
            let (nx, ny) = (nx as usize, ny as usize);
            if grid[ny][nx] == 0 {
                let mx = (x as i32 + dx / 2) as usize;
                let my = (y as i32 + dy / 2) as usize;
                grid[my][mx] = 1;
                carve(grid, nx, ny, active_cols, active_rows, rng);
            }
        }
    }
}

/// Open ~18% of interior walls that connect two path cells -> adds loops / escape routes
fn add_braids(
    grid: &mut Vec<Vec<u8>>,
    active_cols: usize,
    active_rows: usize,
    rng: &mut SmallRng,
    rate: f64,
) {
    for y in 1..active_rows - 1 {
        for x in 1..active_cols - 1 {
            if grid[y][x] == 1 {
                continue;
            }
            let path_neighbours = [(0i32, -1), (0, 1), (-1, 0), (1, 0)]
                .iter()
                .filter(|&&(dx, dy)| {
                    let nx = x as i32 + dx;
                    let ny = y as i32 + dy;
                    nx >= 0
                        && ny >= 0
                        && (nx as usize) < active_cols
                        && (ny as usize) < active_rows
                        && grid[ny as usize][nx as usize] == 1
                })
                .count();
            if path_neighbours == 2 && rng.gen_bool(rate) {
                grid[y][x] = 1;
            }
        }
    }
}
