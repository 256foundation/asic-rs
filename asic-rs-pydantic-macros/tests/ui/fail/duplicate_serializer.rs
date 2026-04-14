#![allow(dead_code)]

#[derive(asic_rs_pydantic::PyPydanticData)]
struct Duplicate {
    #[pydantic_data(to_string, to_string)]
    values: Vec<Nested>,
}

struct Nested;

fn main() {}
