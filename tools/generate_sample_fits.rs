use std::fs;

const CARD_BYTES: usize = 80;
const BLOCK_BYTES: usize = 2880;

fn card(key: &str, value: &str) -> Vec<u8> {
    let mut card = format!("{key:<8}= {value:>20}").into_bytes();
    card.resize(CARD_BYTES, b' ');
    card
}

fn header(mut cards: Vec<Vec<u8>>) -> Vec<u8> {
    let mut end = format!("{:<8}", "END").into_bytes();
    end.resize(CARD_BYTES, b' ');
    cards.push(end);
    let mut bytes = cards.into_iter().flatten().collect::<Vec<_>>();
    bytes.resize(bytes.len().div_ceil(BLOCK_BYTES) * BLOCK_BYTES, b' ');
    bytes
}

fn main() {
    let mut bytes = header(vec![
        card("SIMPLE", "T"),
        card("BITPIX", "16"),
        card("NAXIS", "2"),
        card("NAXIS1", "2"),
        card("NAXIS2", "2"),
        card("EXTEND", "T"),
        card("ORIGIN", "'Goblin++'"),
    ]);
    for value in [1_i16, 2, 3, 4] {
        bytes.extend(value.to_be_bytes());
    }
    bytes.resize(bytes.len().div_ceil(BLOCK_BYTES) * BLOCK_BYTES, 0);

    bytes.extend(header(vec![
        card("XTENSION", "'BINTABLE'"),
        card("BITPIX", "8"),
        card("NAXIS", "2"),
        card("NAXIS1", "20"),
        card("NAXIS2", "3"),
        card("PCOUNT", "0"),
        card("GCOUNT", "1"),
        card("TFIELDS", "3"),
        card("EXTNAME", "'CATALOG'"),
        card("TTYPE1", "'OBJECT'"),
        card("TFORM1", "'8A'"),
        card("TTYPE2", "'Z'"),
        card("TFORM2", "'1D'"),
        card("TTYPE3", "'QUALITY'"),
        card("TFORM3", "'1J'"),
        card("TNULL3", "-999"),
    ]));
    for (name, redshift, quality) in [
        ("GALAXY", 0.125_f64, 3_i32),
        ("QSO", 2.25_f64, 7_i32),
        ("STAR", f64::NAN, -999_i32),
    ] {
        let mut name = name.as_bytes().to_vec();
        name.resize(8, b' ');
        bytes.extend(name);
        bytes.extend(redshift.to_be_bytes());
        bytes.extend(quality.to_be_bytes());
    }
    bytes.resize(bytes.len().div_ceil(BLOCK_BYTES) * BLOCK_BYTES, 0);

    let path = "examples/sample.fits";
    if std::path::Path::new(path).exists() {
        panic!("refusing to overwrite {}", path);
    }
    fs::write(path, bytes).expect("write deterministic FITS fixture");
    println!("created {path}");
}
