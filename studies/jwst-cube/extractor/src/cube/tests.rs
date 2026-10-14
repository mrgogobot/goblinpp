use super::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
static SERIAL: AtomicU64 = AtomicU64::new(0);
fn folder() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "goblin-cube-test-{}-{}",
        std::process::id(),
        SERIAL.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&p).unwrap();
    p
}
fn card(k: &str, v: &str) -> Vec<u8> {
    let s = if k == "END" {
        String::from("END")
    } else {
        format!("{k:<8}= {v}")
    };
    assert!(s.len() <= 80);
    format!("{s:<80}").into_bytes()
}
fn hdu(cards: Vec<(&str, String)>, data: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::new();
    for (k, v) in cards {
        out.extend(card(k, &v));
    }
    out.extend(card("END", ""));
    out.resize(out.len().div_ceil(2880) * 2880, b' ');
    let offset = out.len();
    out.extend(data);
    out.resize(offset + (out.len() - offset).div_ceil(2880) * 2880, 0);
    out
}
fn fixture() -> Vec<u8> {
    let mut out = hdu(
        vec![
            ("SIMPLE", "T".into()),
            ("BITPIX", "8".into()),
            ("NAXIS", "0".into()),
            ("EXTEND", "T".into()),
            ("S_BKDSUB", "'SKIPPED'".into()),
        ],
        vec![],
    );
    for name in ["SCI", "ERR", "DQ", "WMAP"] {
        let mut c = vec![
            ("XTENSION", "'IMAGE'".into()),
            ("BITPIX", if name == "DQ" { "32" } else { "-32" }.into()),
            ("NAXIS", "3".into()),
            ("NAXIS1", "2".into()),
            ("NAXIS2", "3".into()),
            ("NAXIS3", "5".into()),
            ("PCOUNT", "0".into()),
            ("GCOUNT", "1".into()),
            ("EXTNAME", format!("'{name}'")),
        ];
        if name == "SCI" || name == "ERR" {
            c.push(("BUNIT", "'MJy/sr'".into()));
        }
        if name == "SCI" {
            c.extend([
                ("CTYPE3", "'WAVE'".into()),
                ("CUNIT3", "'um'".into()),
                ("CRVAL3", "1".into()),
                ("CRPIX3", "1".into()),
                ("CDELT3", "0.25".into()),
            ]);
        }
        if name == "DQ" {
            c.extend([("BSCALE", "1".into()), ("BZERO", "2147483648".into())]);
        }
        let mut data = Vec::new();
        for k in 0..5 {
            for y in 0..3 {
                for x in 0..2 {
                    if name == "DQ" {
                        let physical = if (x, y, k) == (1, 2, 3) {
                            0x80000000u32
                        } else {
                            0
                        };
                        data.extend((physical ^ 0x80000000).to_be_bytes());
                    } else {
                        let mut v = match name {
                            "SCI" => (100 * k + 10 * y + x) as f32,
                            "ERR" => 2.,
                            _ => 1.,
                        };
                        if name == "SCI" && (x, y, k) == (0, 0, 4) {
                            v = f32::from_bits(0x7fc01234);
                        }
                        if name == "SCI" && (x, y, k) == (1, 0, 0) {
                            v = -0.;
                        }
                        data.extend(v.to_be_bytes());
                    }
                }
            }
        }
        out.extend(hdu(c, data));
    }
    let mut waves = Vec::new();
    for k in 0..5 {
        waves.extend((1. + k as f32 * 0.25).to_be_bytes());
    }
    out.extend(hdu(
        vec![
            ("XTENSION", "'BINTABLE'".into()),
            ("BITPIX", "8".into()),
            ("NAXIS", "2".into()),
            ("NAXIS1", "20".into()),
            ("NAXIS2", "1".into()),
            ("PCOUNT", "0".into()),
            ("GCOUNT", "1".into()),
            ("TFIELDS", "1".into()),
            ("TTYPE1", "'wavelength'".into()),
            ("TFORM1", "'5E'".into()),
            ("EXTNAME", "'WCS-TABLE'".into()),
        ],
        waves,
    ));
    out
}
fn study() -> (PathBuf, String) {
    let p = folder();
    let input = p.join("tiny.fits");
    write_new(&input, &fixture()).unwrap();
    let out = p.join("run");
    let anchor = extract(&input, &hash_file(&input).unwrap(), &out).unwrap();
    (out, anchor)
}
#[test]
fn noncubic_transpose_known_values() {
    let (out, _) = study();
    let b = fs::read(out.join("spectra.bin")).unwrap();
    for y in 0..3 {
        for x in 0..2 {
            for k in 0..5 {
                let i = ((y * 2 + x) * 5 + k) * RECORD;
                let a = fields(&b[i..i + RECORD]);
                if (x, y, k) == (0, 0, 4) {
                    assert_eq!(a[0], 0x7fc01234);
                } else if (x, y, k) == (1, 0, 0) {
                    assert_eq!(a[0], (-0f32).to_bits());
                } else {
                    assert_eq!(f32::from_bits(a[0]), (100 * k + 10 * y + x) as f32);
                }
                assert_eq!(a[1], 2f32.to_bits());
                assert_eq!(
                    a[2],
                    if (x, y, k) == (1, 2, 3) {
                        0x80000000
                    } else {
                        0
                    }
                );
                assert_eq!(a[3], 1f32.to_bits());
            }
        }
    }
}
#[test]
fn full_independent_verification() {
    let (out, a) = study();
    let v = verify(&out, &a).unwrap();
    assert_eq!(v["full_voxels_compared"], 30);
    assert_eq!(v["raw_fields_compared"], 120);
    assert_eq!(v["csv_rows_verified"], 30);
}
#[test]
fn no_channels_dropped_and_nonfinite_blank() {
    let (out, _) = study();
    let s = fs::read_to_string(out.join("spaxel_spectra.csv")).unwrap();
    assert_eq!(s.lines().count(), 31);
    assert!(s.contains("0,0,4,2,,2,0,1,0,0,1"));
    assert!(s.contains("1,0,0,1,-0,2,0,1,1,1,0"));
}
#[test]
fn dq_unsigned_and_nan_payload_preserved() {
    let r = record(
        0x7fc05555u32.to_be_bytes(),
        1f32.to_be_bytes(),
        0x7fffffffu32.to_be_bytes(),
        1f32.to_be_bytes(),
    );
    assert_eq!(fields(&r)[0], 0x7fc05555);
    assert_eq!(fields(&r)[2], u32::MAX);
    assert_eq!(fields(&r)[4], 9);
}
#[test]
fn quality_masks_do_not_erase_values() {
    assert_eq!(flags(10., -1., 2, 0.), 4 | 8 | 32);
    assert!(!science_ok(flags(10., 1., 2, 1.)));
    assert!(science_ok(flags(10., -1., 0, 1.)));
    assert!(!uncertainty_ok(flags(10., -1., 0, 1.)));
    assert!(!uncertainty_ok(flags(10., 0., 0, 1.)));
    assert_eq!(flags(f32::INFINITY, f32::NAN, 0, f32::NAN), 1 | 2 | 16);
}
#[test]
fn linear_wavelength_one_based_crpix() {
    let f = FitsFile::from_bytes("fixture".into(), fixture()).unwrap();
    let l = layout(&f).unwrap();
    assert_eq!(l.wavelengths, vec![1., 1.25, 1.5, 1.75, 2.]);
}
#[test]
fn bad_axis_shape_refused() {
    let mut f = FitsFile::from_bytes("fixture".into(), fixture()).unwrap();
    f.hdus[2].axes[1] = 2;
    assert!(layout(&f).is_err());
}
#[test]
fn nonlinear_or_coupled_wcs_refused() {
    let mut f = FitsFile::from_bytes("fixture".into(), fixture()).unwrap();
    f.hdus[1]
        .header
        .insert("CTYPE3".into(), HeaderValue::Text("WAVE-TAB".into()));
    assert!(layout(&f).is_err());
    f.hdus[1]
        .header
        .insert("CTYPE3".into(), HeaderValue::Text("WAVE".into()));
    f.hdus[1]
        .header
        .insert("PC3_1".into(), HeaderValue::Float(0.1));
    assert!(layout(&f).is_err());
}
#[test]
fn scaling_and_bad_units_refused() {
    let mut f = FitsFile::from_bytes("fixture".into(), fixture()).unwrap();
    f.hdus[1]
        .header
        .insert("BSCALE".into(), HeaderValue::Float(2.));
    assert!(layout(&f).is_err());
    f.hdus[1].header.remove("BSCALE");
    f.hdus[1]
        .header
        .insert("BUNIT".into(), HeaderValue::Text("Jy".into()));
    assert!(layout(&f).is_err());
}
#[test]
fn input_pin_and_overwrite_refused() {
    let p = folder();
    let input = p.join("tiny.fits");
    write_new(&input, &fixture()).unwrap();
    assert!(extract(&input, &"0".repeat(64), &p.join("bad")).is_err());
    assert!(extract(&input, &hash_file(&input).unwrap(), &p).is_err());
}
#[test]
fn tamper_and_anchor_refused() {
    let (out, a) = study();
    assert!(verify(&out, &"0".repeat(64)).is_err());
    let mut b = fs::read(out.join("spectra.bin")).unwrap();
    b[0] ^= 1;
    fs::write(out.join("spectra.bin"), b).unwrap();
    assert!(verify(&out, &a).is_err());
}
#[test]
fn spectrum_export_and_bounds() {
    let (out, a) = study();
    let p = out.join("one.csv");
    spectrum(&out, &a, 1, 2, &p).unwrap();
    assert_eq!(fs::read_to_string(&p).unwrap().lines().count(), 6);
    assert!(spectrum(&out, &a, 2, 0, &out.join("bad.csv")).is_err());
    assert!(spectrum(&out, &a, 1, 2, &p).is_err());
}
#[test]
fn truncated_input_refused() {
    let mut b = fixture();
    b.truncate(b.len() - 2880);
    assert!(FitsFile::from_bytes("truncated".into(), b).is_err());
}

#[test]
fn repeated_verification_validates_without_overwrite() {
    let (out, anchor) = study();
    let value = verify(&out, &anchor).unwrap();
    save_verification(&out, &value).unwrap();
    let original = fs::read(out.join("verification.json")).unwrap();
    save_verification(&out, &value).unwrap();
    assert_eq!(original, fs::read(out.join("verification.json")).unwrap());
    let mut wrong = value.clone();
    wrong["status"] = json!("INVENTED");
    assert!(save_verification(&out, &wrong).is_err());
}
