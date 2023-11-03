#![forbid(unsafe_code)]

////////////////////////////////////////////////////////////////////////////////

static DIFF: &[(isize, isize); 8] = &[
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (0, -1),
    (0, 1),
    (1, -1),
    (1, 0),
    (1, 1),
];

struct Neighbour {
    grid_rows: usize,
    grid_cols: usize,

    row: usize,
    col: usize,

    idx: usize,
}

impl Neighbour {
    pub fn new(grid_rows: usize, grid_cols: usize, row: usize, col: usize) -> Self {
        if row >= grid_rows || col >= grid_cols {
            panic!("({row}, {col}) index is outside of grid!")
        }

        Neighbour {
            grid_rows,
            grid_cols,
            row,
            col,
            idx: 0,
        }
    }
}

impl Iterator for Neighbour {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        while self.idx < DIFF.len() {
            let (drow, dcol) = DIFF[self.idx];
            self.idx += 1;

            let new_row = self.row as isize + drow;
            let new_col = self.col as isize + dcol;

            if new_row >= 0
                && new_row < self.grid_rows as isize
                && new_col >= 0
                && new_col < self.grid_cols as isize
            {
                return Some((new_row as usize, new_col as usize));
            }
        }

        None
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Grid<T> {
    rows: usize,
    cols: usize,
    grid: Vec<T>,
}

impl<T: Clone + Default> Grid<T> {
    pub fn new(rows: usize, cols: usize) -> Self {
        Grid {
            rows,
            cols,
            grid: vec![Default::default(); rows * cols],
        }
    }

    pub fn from_slice(grid: &[T], rows: usize, cols: usize) -> Self {
        let size = grid.len();
        if size != rows * cols {
            panic!("inconsistent data: grid size ({size}) is not equal to {rows} * {cols}")
        }

        Grid {
            rows,
            cols,
            grid: grid.to_vec(),
        }
    }

    pub fn size(&self) -> (usize, usize) {
        (self.rows, self.cols)
    }

    pub fn get(&self, row: usize, col: usize) -> &T {
        &self.grid[row * self.rows + col]
    }

    pub fn set(&mut self, value: T, row: usize, col: usize) {
        self.grid[row * self.rows + col] = value
    }

    pub fn neighbours(&self, row: usize, col: usize) -> impl Iterator<Item = (usize, usize)> {
        Neighbour::new(self.rows, self.cols, row, col)
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Dead,
    Alive,
}

impl Default for Cell {
    fn default() -> Self {
        Self::Dead
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(PartialEq, Eq)]
pub struct GameOfLife {
    grid: Grid<Cell>,
    changes: Grid<bool>,
}

impl GameOfLife {
    pub fn from_grid(grid: Grid<Cell>) -> Self {
        GameOfLife {
            changes: Grid::new(grid.rows, grid.cols),
            grid,
        }
    }

    pub fn get_grid(&self) -> &Grid<Cell> {
        &self.grid
    }

    fn get_n_alive_neighbours(&self, row: usize, col: usize) -> usize {
        self.grid
            .neighbours(row, col)
            .filter(|(n_row, n_col)| self.grid.get(*n_row, *n_col) == &Cell::Alive)
            .count()
    }

    fn detect_changes_in_cell(&mut self, row: usize, col: usize) {
        let n_alive_neighbours = self.get_n_alive_neighbours(row, col);

        match self.grid.get(row, col) {
            Cell::Alive if !(2..=3).contains(&n_alive_neighbours) => {
                self.changes.set(true, row, col)
            }

            Cell::Dead if n_alive_neighbours == 3 => self.changes.set(true, row, col),

            _ => {}
        }
    }

    fn detect_changes(&mut self) {
        for row in 0..self.grid.rows {
            for col in 0..self.grid.cols {
                self.detect_changes_in_cell(row, col)
            }
        }
    }

    fn apply_changes(&mut self) {
        for row in 0..self.grid.rows {
            for col in 0..self.grid.cols {
                if !self.changes.get(row, col) {
                    continue;
                }

                match self.grid.get(row, col) {
                    Cell::Alive => self.grid.set(Cell::Dead, row, col),
                    Cell::Dead => self.grid.set(Cell::Alive, row, col),
                }

                self.changes.set(false, row, col);
            }
        }
    }

    pub fn step(&mut self) {
        self.detect_changes();
        self.apply_changes();
    }
}
