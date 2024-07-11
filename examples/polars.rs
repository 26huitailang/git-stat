use polars::prelude::*;

fn main() {
    let path = "detail.csv";
    let q = LazyCsvReader::new(path)
        .with_has_header(true)
        .finish()
        .unwrap()
        .select(vec![
            col("repo"),
            col("branch"),
            col("author"),
            col("insertions"),
            col("deletions"),
        ])
        .group_by(vec![col("repo"), col("branch"), col("author")])
        .agg([col("*").sum()]);

    let df = q.collect().unwrap();

    println!("{}", df)
}
