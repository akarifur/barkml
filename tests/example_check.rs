#[test]
fn example_bml_parses_with_interpolation() {
    let src = include_str!("../examples/example.bml");
    let module = barkml::from_str(src).expect("example.bml parses");
    let value = module
        .find_by_path("section-1.interpolated")
        .expect("interpolated exists")
        .get_value()
        .expect("has value");
    assert_eq!(
        value.as_string().map(String::as_str),
        Some("foobar from section-1")
    );
}
