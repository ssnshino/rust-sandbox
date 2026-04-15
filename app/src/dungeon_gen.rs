use rand::seq::SliceRandom;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use rand::Rng;

pub const COLS: usize = 41;
pub const ROWS: usize = 41;

/// Recursive-backtracker maze + braid (extra loops for escape routes)
pub fn generate(seed: u64) -> Vec<Vec<u8>> {
    let mut grid = vec![vec![0u8; COLS]; ROWS];
    let mut rng = SmallRng::seed_from_u64(seed);
    carve(&mut grid, 1, 1, &mut rng);
    add_braids(&mut grid, &mut rng, 0.18);
    grid
}

fn carve(grid: &mut Vec<Vec<u8>>, x: usize, y: usize, rng: &mut SmallRng) {
    grid[y][x] = 1;
    let mut dirs: [(i32, i32); 4] = [(0, -2), (0, 2), (-2, 0), (2, 0)];
    dirs.shuffle(rng);
    for (dx, dy) in dirs {
        let nx = x as i32 + dx;
        let ny = y as i32 + dy;
        if nx > 0 && ny > 0 && (nx as usize) < COLS - 1 && (ny as usize) < ROWS - 1 {
            let (nx, ny) = (nx as usize, ny as usize);
            if grid[ny][nx] == 0 {
                let mx = (x as i32 + dx / 2) as usize;
                let my = (y as i32 + dy / 2) as usize;
                grid[my][mx] = 1;
                carve(grid, nx, ny, rng);
            }
        }
    }
}

/// Open ~18% of interior walls that connect two path cells -> adds loops / escape routes
fn add_braids(grid: &mut Vec<Vec<u8>>, rng: &mut SmallRng, rate: f64) {
    for y in 1..ROWS - 1 {
        for x in 1..COLS - 1 {
            if grid[y][x] == 1 { continue; }
            // count path-cell neighbours
            let path_neighbours = [(0i32,-1),(0,1),(-1,0),(1,0)].iter()
                .filter(|&&(dx,dy)| {
                    let nx = x as i32 + dx; let ny = y as i32 + dy;
                    nx>=0&&ny>=0&&(nx as usize)<COLS&&(ny as usize)<ROWS
                        &&grid[ny as usize][nx as usize]==1
                }).count();
            // only knock down walls that connect exactly 2 paths (true shortcuts)
            if path_neighbours == 2 && rng.gen_bool(rate) {
                grid[y][x] = 1;
            }
        }
    }
}
