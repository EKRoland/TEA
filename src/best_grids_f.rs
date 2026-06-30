use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Root {
    #[serde(alias = "best")]
    pub best_grids: Vec<GridBlock>,
}

#[derive(Debug, Deserialize)]
pub struct GridBlock {
    pub rows: usize,
    pub cols: usize,
    pub solution: Vec<Solution>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Solution {
    pub name: String,
    pub steps: u64,
    pub ant: [isize; 3],
    pub grid: Vec<String>,
}




