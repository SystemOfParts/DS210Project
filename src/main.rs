use std::error::Error;
use std::env;

extern crate petgraph;
extern crate csv;
extern crate plotters;

mod graph {
    use std::error::Error;
    use std::path::Path;
    use std::collections::{HashMap, HashSet};
    use petgraph::graphmap::DiGraphMap;
    use csv::ReaderBuilder;

    // This function loads a directed graph from a TSV file with the source and the target in columns 0 and 1, respectively.
    // Through this, it maps the subreddit names to the integer IDs.
    pub fn load_graph<P: AsRef<Path>>(path: P) -> Result<DiGraphMap<usize, ()>, Box<dyn Error>> {
        let mut rdr = ReaderBuilder::new()
            .delimiter(b'\t')
            .has_headers(false)
            .from_path(path)?;
        let mut graph = DiGraphMap::<usize, ()>::new();
        let mut id_map: HashMap<String, usize> = HashMap::new();
        let mut next_id: usize = 0;

        for result in rdr.records() {
            let record = result?;
            let source_str = record.get(0).unwrap().to_string();
            let target_str = record.get(1).unwrap().to_string();

            let source_id = *id_map.entry(source_str.clone()).or_insert_with(|| {
                let id = next_id;
                next_id += 1;
                id
            });
            let target_id = *id_map.entry(target_str.clone()).or_insert_with(|| {
                let id = next_id;
                next_id += 1;
                id
            });
            graph.add_edge(source_id, target_id, ());
        }
        Ok(graph)
    }

    // This function computes the out-degree distribution: Vector of (degree, count).
    pub fn degree_distribution(graph: &DiGraphMap<usize, ()>) -> Vec<(usize, usize)> {
        let mut counts: HashMap<usize, usize> = HashMap::new();
        for node in graph.nodes() {
            let deg = graph.neighbors(node).count();
            *counts.entry(deg).or_insert(0) += 1;
        }
        let mut dist: Vec<(usize, usize)> = counts.into_iter().collect();
        dist.sort_by_key(|&(deg, _)| deg);
        dist
    }

    // This function computes the distribution of the number of second-hop neighbors (distance 2), to see if the degrees matter in distribution.
    pub fn distance2_distribution(graph: &DiGraphMap<usize, ()>) -> Vec<(usize, usize)> {
        let mut counts: HashMap<usize, usize> = HashMap::new();
        for node in graph.nodes() {
            let mut second: HashSet<usize> = HashSet::new();
            // First-hop neighbors
            for n1 in graph.neighbors(node) {
                // Second-hop neighbors
                for n2 in graph.neighbors(n1) {
                    if n2 != node && !graph.neighbors(node).any(|x| x == n2) {
                        second.insert(n2);
                    }
                }
            }
            let cnt2 = second.len();
            *counts.entry(cnt2).or_insert(0) += 1;
        }
        let mut dist: Vec<(usize, usize)> = counts.into_iter().collect();
        dist.sort_by_key(|&(cnt, _)| cnt);
        dist
    }
}

mod visualization {
    use std::error::Error;
    use plotters::prelude::*;
    use plotters::coord::combinators::IntoLogRange;

    // This just plots a histogram (Common bar graph).
    // I stopped the axes at a certain point to help visualization
    pub fn plot_histogram_custom(
        data: &[(usize, usize)],
        output: &str,
        title: &str,
        x_limit: u32,
        y_limit: u32,
        log_y: bool,
    ) -> Result<(), Box<dyn Error>> {
        let root = BitMapBackend::new(output, (1024, 768)).into_drawing_area();
        root.fill(&WHITE)?;

        if log_y {
            let max_count = data.iter().map(|&(_, y)| y).max().unwrap_or(1) as u32;
            let y_range = (1u32..(max_count + 1)).log_scale();

            let mut chart = ChartBuilder::on(&root)
                .margin(20)
                .caption(title, ("sans-serif", 40).into_font())
                .x_label_area_size(40)
                .y_label_area_size(40)
                .build_cartesian_2d(0u32..x_limit, y_range)?;
            chart.configure_mesh().draw()?;

            for &(x, y) in data.iter() {
                let y_val = (y.max(1)) as u32;
                chart.draw_series(std::iter::once(
                    Rectangle::new(
                        [(x as u32, 1u32), ((x + 1) as u32, y_val)],
                        BLUE.filled(),
                    ),
                ))?;
            }
        } else {
            let mut chart = ChartBuilder::on(&root)
                .margin(20)
                .caption(title, ("sans-serif", 40).into_font())
                .x_label_area_size(40)
                .y_label_area_size(40)
                .build_cartesian_2d(0u32..x_limit, 0u32..y_limit)?;
            chart.configure_mesh().draw()?;

            for &(x, y) in data.iter() {
                chart.draw_series(std::iter::once(
                    Rectangle::new(
                        [(x as u32, 0u32), ((x + 1) as u32, y as u32)],
                        BLUE.filled(),
                    ),
                ))?;
            }
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <path to TSV file>", args[0]);
        std::process::exit(1);
    }
    let path = &args[1];

    println!("Loading graph from {}...", path);
    let graph = graph::load_graph(path)?;
    println!("Loaded graph: {} nodes, {} edges", graph.node_count(), graph.edge_count());


    // Degree distribution of the X axis up to 100, and the Y axis being logarithmic (Made sense for this visualization)
    println!("Computing degree distribution...");
    let deg_dist = graph::degree_distribution(&graph);
    visualization::plot_histogram_custom(
        &deg_dist,
        "degree_distribution.png",
        "Degree Distribution",
        100,
        0,
        true,
    )?;
    println!("Saved degree_distribution.png");

    // Distance-2 (Two degree) distribution, the x and y axes both go up to 500 for visualization purposes
    println!("Computing distance-2 neighbor distribution...");
    let dist2_dist = graph::distance2_distribution(&graph);
    visualization::plot_histogram_custom(
        &dist2_dist,
        "distance2_distribution.png",
        "Distance-2 Distribution",
        500,
        500,
        false,
    )?;
    println!("Saved distance2_distribution.png");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::graph::{degree_distribution, distance2_distribution};
    use petgraph::graphmap::DiGraphMap;

    #[test]
    // This tests the degree distribution by making a small graph and asserting to see if the expected outcome matches the actual outcome
    fn test_degree_distribution() {
        let mut g = DiGraphMap::<usize, ()>::new();
        g.add_node(0);
        g.add_node(1);
        g.add_node(2);
        g.add_node(3);
        g.add_edge(0, 1, ());
        g.add_edge(1, 2, ());

        let dist = degree_distribution(&g);
        let expected = vec![(0, 2), (1, 2)];
        assert_eq!(dist, expected);
    }

    #[test]
    // Same for this test, although it's with distance 2.
    fn test_distance2_distribution() {
        let mut g = DiGraphMap::<usize, ()>::new();
        g.add_node(0);
        g.add_node(1);
        g.add_node(2);
        g.add_node(3);
        g.add_edge(0, 1, ());
        g.add_edge(1, 2, ());

        let dist2 = distance2_distribution(&g);
        let expected2 = vec![(0, 3), (1, 1)];
        assert_eq!(dist2, expected2);
    }
}