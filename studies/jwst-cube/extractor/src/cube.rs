use goblinpp::fits::{FitsFile, FitsHdu, HduKind, HeaderValue};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const RECORD: usize = 20;
const TILE: usize = 16;
const EXPECTED: &str = "18b78fe42f03e2ed886b94adc582ba7bb2857ad78c4edb6e55a29fc8d6c995b1";
const CSV_HEADER: &str = "x0,y0,channel0,wavelength_um,sci_mjy_sr,err_mjy_sr,dq,wmap,science_usable,uncertainty_usable,status_bits";

fn need(ok: bool, msg: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(msg.into()) }
}
fn hash_bytes(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn hash_file(p: &Path) -> Result<String> {
    let mut f = File::open(p)?;
    let mut d = Sha256::new();
    let mut b = [0u8; 65536];
    loop {
        let n = f.read(&mut b)?;
        if n == 0 {
            break;
        }
        d.update(&b[..n]);
    }
    Ok(format!("{:x}", d.finalize()))
}
fn new_file(p: &Path) -> Result<File> {
    Ok(OpenOptions::new().write(true).create_new(true).open(p)?)
}
fn write_new(p: &Path, b: &[u8]) -> Result<()> {
    let mut f = new_file(p)?;
    f.write_all(b)?;
    f.sync_all()?;
    Ok(())
}
fn number(h: &FitsHdu, k: &str) -> Result<f64> {
    let n = h
        .header
        .get(k)
        .and_then(HeaderValue::as_f64)
        .ok_or_else(|| format!("Missing numeric {k}"))?;
    need(n.is_finite(), "Nonfinite header value")?;
    Ok(n)
}
fn optional(h: &FitsHdu, k: &str, default: f64) -> Result<f64> {
    if h.header.contains_key(k) {
        number(h, k)
    } else {
        Ok(default)
    }
}
fn text<'a>(h: &'a FitsHdu, k: &str) -> Result<&'a str> {
    match h.header.get(k) {
        Some(HeaderValue::Text(s)) => Ok(s.trim()),
        _ => Err(format!("Missing text {k}").into()),
    }
}
#[derive(Clone)]
struct Layout {
    nx: usize,
    ny: usize,
    nz: usize,
    hdus: [FitsHdu; 4],
    wavelengths: Vec<f64>,
    meta: Value,
}
fn layout(f: &FitsFile) -> Result<Layout> {
    let ext = |name: &str| -> Result<FitsHdu> {
        let v: Vec<_> = f
            .hdus
            .iter()
            .filter(|h| h.extension_name.as_deref() == Some(name))
            .collect();
        need(v.len() == 1, "Required extension missing or duplicated")?;
        Ok(v[0].clone())
    };
    let hdus = [ext("SCI")?, ext("ERR")?, ext("DQ")?, ext("WMAP")?];
    let s = &hdus[0];
    need(s.axes.len() == 3, "SCI must have exactly three axes")?;
    let (nx, ny, nz) = (s.axes[0], s.axes[1], s.axes[2]);
    need(
        nx > 0 && ny > 0 && nz > 0 && nx <= 512 && ny <= 512 && nz <= 20000,
        "Cube dimensions outside tested resource bounds",
    )?;
    let count = nx
        .checked_mul(ny)
        .and_then(|n| n.checked_mul(nz))
        .ok_or("Cube count overflow")?;
    need(count <= 100_000_000, "Cube exceeds 100 million voxel bound")?;
    for (j, h) in hdus.iter().enumerate() {
        need(
            h.kind == HduKind::Image && h.axes == s.axes,
            "Mismatched or non-image science extensions",
        )?;
        need(h.data_length == count as u64 * 4, "Unexpected data length")?;
        need(
            h.bitpix == if j == 2 { 32 } else { -32 },
            "Only f32 SCI/ERR/WMAP and i32-backed unsigned DQ are supported",
        )?;
        need(
            optional(h, "BSCALE", 1.)? == 1.,
            "Scaled images are not supported by this narrow extractor",
        )?;
        need(
            optional(h, "BZERO", 0.)? == if j == 2 { 2147483648. } else { 0. },
            "Unexpected image zero point",
        )?;
        need(
            !h.header.contains_key("BLANK"),
            "BLANK image convention not implemented",
        )?;
    }
    need(
        text(s, "BUNIT")? == "MJy/sr" && text(&hdus[1], "BUNIT")? == "MJy/sr",
        "Unexpected SCI/ERR units",
    )?;
    need(
        text(s, "CTYPE3")? == "WAVE" && text(s, "CUNIT3")? == "um",
        "Only linear WAVE in micrometres is implemented; WAVE-TAB is refused",
    )?;
    for k in ["PC1_3", "PC2_3", "PC3_1", "PC3_2"] {
        need(
            optional(s, k, 0.)? == 0.,
            "Coupled spatial/spectral WCS is not implemented",
        )?;
    }
    let crpix = number(s, "CRPIX3")?;
    let crval = number(s, "CRVAL3")?;
    let step = number(s, "CDELT3")? * optional(s, "PC3_3", 1.)?;
    need(step > 0., "Wavelength increment must be positive")?;
    let wavelengths: Vec<_> = (0..nz)
        .map(|k| crval + ((k + 1) as f64 - crpix) * step)
        .collect();
    need(
        wavelengths.iter().all(|x| x.is_finite() && *x > 0.),
        "Invalid wavelength range",
    )?;
    let t = ext("WCS-TABLE")?;
    let cols = t.columns();
    let c = cols
        .iter()
        .find(|c| c.name == "wavelength")
        .ok_or("Missing wavelength table column")?;
    need(
        t.rows == Some(1) && c.repeat == nz && c.element_type == "f32",
        "Unexpected wavelength table structure",
    )?;
    let mut max_diff = 0f64;
    for (k, &w) in wavelengths.iter().enumerate() {
        let goblinpp::fits::ColumnValue::Number(tw) =
            f.column_value(t.index, "wavelength", 0, Some(k))?
        else {
            return Err("Missing table wavelength".into());
        };
        max_diff = max_diff.max((tw - w).abs());
        need(
            (tw - w).abs() <= 2. * f32::EPSILON as f64 * w.abs().max(1.),
            "Header/table wavelength discrepancy exceeds two relative float32 epsilons",
        )?;
    }
    let meta = json!({"fits_axis_order":["x","y","wavelength"],"dimensions":[nx,ny,nz],"science_unit":"MJy/sr","error_unit":"MJy/sr","wavelength_unit":"um","spectral_wcs":"WAVE","crpix3":crpix,"crval3":crval,"step_um":step,"wavelength_authority":"SCI linear FITS WCS; WCS-TABLE is a consistency check, not substituted","wavelength_table_max_difference_um":max_diff,"spatial_wcs_cards":s.cards.iter().filter(|c| c.keyword.starts_with("PC")||c.keyword.starts_with("CR")||c.keyword.starts_with("CD")||c.keyword.starts_with("CTYP")||c.keyword.starts_with("CUNIT")||c.keyword.starts_with("PIXAR")).cloned().collect::<Vec<_>>(),"background_subtraction_header":f.hdus[0].header.get("S_BKDSUB"),"srctype":s.header.get("SRCTYPE")});
    Ok(Layout {
        nx,
        ny,
        nz,
        hdus,
        wavelengths,
        meta,
    })
}

// Flags annotate, never erase raw samples. Zero error is retained, but not suitable for uncertainty weighting.
fn flags(s: f32, e: f32, d: u32, w: f32) -> u32 {
    u32::from(!s.is_finite())
        | (u32::from(!e.is_finite()) << 1)
        | (u32::from(e.is_finite() && e < 0.) << 2)
        | (u32::from(d != 0) << 3)
        | (u32::from(!w.is_finite()) << 4)
        | (u32::from(w.is_finite() && w <= 0.) << 5)
        | (u32::from(e == 0.) << 6)
}
fn science_ok(f: u32) -> bool {
    f & (1 | 8 | 16 | 32) == 0
}
fn uncertainty_ok(f: u32) -> bool {
    science_ok(f) && f & (2 | 4 | 64) == 0
}
fn record(s: [u8; 4], e: [u8; 4], d: [u8; 4], w: [u8; 4]) -> [u8; RECORD] {
    let sb = u32::from_be_bytes(s);
    let eb = u32::from_be_bytes(e);
    let wb = u32::from_be_bytes(w);
    let physical_dq = (i32::from_be_bytes(d) as i64 + 2147483648) as u32;
    let fs = flags(
        f32::from_bits(sb),
        f32::from_bits(eb),
        physical_dq,
        f32::from_bits(wb),
    );
    let mut out = [0; RECORD];
    for (j, n) in [sb, eb, physical_dq, wb, fs].iter().enumerate() {
        out[j * 4..j * 4 + 4].copy_from_slice(&n.to_le_bytes());
    }
    out
}
fn fields(r: &[u8]) -> [u32; 5] {
    std::array::from_fn(|j| u32::from_le_bytes(r[j * 4..j * 4 + 4].try_into().unwrap()))
}
fn value(n: u32) -> String {
    let v = f32::from_bits(n) as f64;
    if v.is_finite() {
        v.to_string()
    } else {
        String::new()
    }
}
fn csv_row(x: usize, y: usize, k: usize, w: f64, r: &[u8]) -> String {
    let [s, e, d, m, f] = fields(r);
    format!(
        "{x},{y},{k},{w},{},{},{d},{},{},{},{f}\n",
        value(s),
        value(e),
        value(m),
        u8::from(science_ok(f)),
        u8::from(uncertainty_ok(f))
    )
}
fn read_plane(f: &mut File, h: &FitsHdu, k: usize, n: usize) -> Result<Vec<u8>> {
    let mut b = vec![0; n * 4];
    f.seek(SeekFrom::Start(h.data_offset + (k * n * 4) as u64))?;
    f.read_exact(&mut b)?;
    Ok(b)
}
fn extract_images(input: &Path, out: &Path, l: &Layout) -> Result<()> {
    let mut input = File::open(input)?;
    let mut binary = new_file(&out.join("spectra.bin"))?;
    let plane = l.nx * l.ny;
    binary.set_len((plane * l.nz * RECORD) as u64)?;
    // At most 16 planes are buffered for transpose; each FITS image payload is read exactly once.
    let mut tile = vec![0u8; plane * TILE * RECORD];
    let mut stripe = vec![0u8; TILE * RECORD];
    for z0 in (0..l.nz).step_by(TILE) {
        let nk = TILE.min(l.nz - z0);
        for dz in 0..nk {
            let p: Vec<_> = l
                .hdus
                .iter()
                .map(|h| read_plane(&mut input, h, z0 + dz, plane))
                .collect::<Result<_>>()?;
            for xy in 0..plane {
                let b = xy * 4;
                let r = record(
                    p[0][b..b + 4].try_into()?,
                    p[1][b..b + 4].try_into()?,
                    p[2][b..b + 4].try_into()?,
                    p[3][b..b + 4].try_into()?,
                );
                let offset = (dz * plane + xy) * RECORD;
                tile[offset..offset + RECORD].copy_from_slice(&r);
            }
        }
        for xy in 0..plane {
            for dz in 0..nk {
                stripe[dz * RECORD..(dz + 1) * RECORD].copy_from_slice(
                    &tile[(dz * plane + xy) * RECORD..(dz * plane + xy + 1) * RECORD],
                );
            }
            binary.seek(SeekFrom::Start(((xy * l.nz + z0) * RECORD) as u64))?;
            binary.write_all(&stripe[..nk * RECORD])?;
        }
    }
    binary.sync_all()?;
    Ok(())
}
fn write_tables(out: &Path, l: &Layout) -> Result<Value> {
    let mut reader = BufReader::new(File::open(out.join("spectra.bin"))?);
    let mut all = BufWriter::new(new_file(&out.join("spaxel_spectra.csv"))?);
    let mut spatial = BufWriter::new(new_file(&out.join("spaxels.csv"))?);
    let mut waves = BufWriter::new(new_file(&out.join("wavelengths.csv"))?);
    writeln!(all, "{CSV_HEADER}")?;
    writeln!(
        spatial,
        "x0,y0,spaxel_id0,binary_offset_bytes,channels,science_usable_channels,uncertainty_usable_channels"
    )?;
    writeln!(
        waves,
        "channel0,wavelength_um,science_usable_spaxels,uncertainty_usable_spaxels"
    )?;
    let mut channel_good = vec![[0u64; 2]; l.nz];
    let mut statuses = BTreeMap::<u32, u64>::new();
    let mut dqs = BTreeMap::<u32, u64>::new();
    let mut good_total = 0u64;
    let mut err_total = 0u64;
    let mut covered = 0u64;
    for xy in 0..l.nx * l.ny {
        let (x, y) = (xy % l.nx, xy / l.nx);
        let mut counts = [0u64; 2];
        for (k, channel) in channel_good.iter_mut().enumerate() {
            let mut r = [0; RECORD];
            reader.read_exact(&mut r)?;
            let [_, _, dq, _, f] = fields(&r);
            *statuses.entry(f).or_default() += 1;
            *dqs.entry(dq).or_default() += 1;
            if science_ok(f) {
                counts[0] += 1;
                channel[0] += 1;
            }
            if uncertainty_ok(f) {
                counts[1] += 1;
                channel[1] += 1;
            }
            all.write_all(csv_row(x, y, k, l.wavelengths[k], &r).as_bytes())?;
        }
        good_total += counts[0];
        err_total += counts[1];
        covered += u64::from(counts[0] > 0);
        writeln!(
            spatial,
            "{x},{y},{xy},{},{},{},{}",
            xy * l.nz * RECORD,
            l.nz,
            counts[0],
            counts[1]
        )?;
    }
    for (k, c) in channel_good.iter().enumerate() {
        writeln!(waves, "{k},{},{},{}", l.wavelengths[k], c[0], c[1])?;
    }
    all.flush()?;
    spatial.flush()?;
    waves.flush()?;
    Ok(
        json!({"spatial_pixels":l.nx*l.ny,"channels_per_spectrum":l.nz,"total_voxels":l.nx*l.ny*l.nz,"spaxels_with_science_usable_samples":covered,"science_usable_voxels":good_total,"uncertainty_usable_voxels":err_total,"status_histogram":statuses,"dq_histogram":dqs}),
    )
}
fn copy_snapshot(src: &Path, dst: &Path, expected: &str) -> Result<()> {
    need(
        fs::symlink_metadata(src)?.is_file()
            && !fs::symlink_metadata(src)?.file_type().is_symlink(),
        "Input must be a regular non-symlink file",
    )?;
    need(
        fs::metadata(src)?.len() <= 1024 * 1024 * 1024,
        "Input over 1 GiB",
    )?;
    let mut input = File::open(src)?;
    let mut output = new_file(dst)?;
    let mut d = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = input.read(&mut buf)?;
        if n == 0 {
            break;
        }
        output.write_all(&buf[..n])?;
        d.update(&buf[..n]);
    }
    output.sync_all()?;
    need(
        format!("{:x}", d.finalize()) == expected,
        "Copied input SHA-256 does not match independently pinned cube hash",
    )?;
    need(
        hash_file(src)? == expected,
        "Original input changed during snapshot",
    )?;
    Ok(())
}
fn extract(src: &Path, expected: &str, out: &Path) -> Result<String> {
    need(
        expected.len() == 64
            && expected
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
        "Expected lowercase SHA-256 required",
    )?;
    fs::create_dir(out)?;
    copy_snapshot(src, &out.join("input.fits"), expected)?;
    let f = FitsFile::open(out.join("input.fits"))?;
    need(f.sha256 == expected, "Snapshot digest mismatch")?;
    let l = layout(&f)?;
    extract_images(&out.join("input.fits"), out, &l)?;
    let summary = write_tables(out, &l)?;
    write_new(
        &out.join("metadata.json"),
        &serde_json::to_vec_pretty(&l.meta)?,
    )?;
    write_new(&out.join("extractor_cube.rs"), include_bytes!("cube.rs"))?;
    write_new(&out.join("extractor_main.rs"), include_bytes!("main.rs"))?;
    write_new(
        &out.join("extractor_Cargo.toml"),
        include_bytes!("../Cargo.toml"),
    )?;
    write_new(
        &out.join("extractor_Cargo.lock"),
        include_bytes!("../Cargo.lock"),
    )?;
    write_new(
        &out.join("extractor_tests.rs"),
        include_bytes!("cube/tests.rs"),
    )?;
    need(
        hash_file(&out.join("input.fits"))? == expected && hash_file(src)? == expected,
        "Input changed during extraction",
    )?;
    let mut outputs = BTreeMap::new();
    for name in [
        "input.fits",
        "spectra.bin",
        "spaxel_spectra.csv",
        "spaxels.csv",
        "wavelengths.csv",
        "metadata.json",
        "extractor_cube.rs",
        "extractor_main.rs",
        "extractor_Cargo.toml",
        "extractor_Cargo.lock",
        "extractor_tests.rs",
    ] {
        let p = out.join(name);
        outputs.insert(
            name,
            json!({"bytes":fs::metadata(&p)?.len(),"sha256":hash_file(&p)?}),
        );
    }
    let receipt = json!({"schema":"goblin.study.cube-extraction.v1","status":"EXTRACTED_PENDING_INDEPENDENT_VERIFICATION","input_path":src.to_string_lossy(),"input_sha256":expected,"metadata":l.meta,"summary":summary,"outputs":outputs,"engine":"standalone Rust extraction driver using Goblin++ alpha.35 FITS metadata reader; not a new language builtin or standard .gbl run receipt","goblin_version":goblinpp::VERSION,"goblin_source_commit":"6a1e911bdce2254256df04c4adef62f6cc00e100","tool_binary_sha256":hash_file(&std::env::current_exe()?)?,"toolchain":"rustc 1.92.0","policy":{"regions":"All spaxels, no spatial coaddition","science_usable":"SCI finite AND DQ == 0 AND WMAP finite and > 0; conservative annotation only","uncertainty_usable":"science_usable AND ERR finite and > 0","status_bits":{"1":"SCI nonfinite","2":"ERR nonfinite","4":"ERR negative","8":"DQ nonzero (all original bits retained)","16":"WMAP nonfinite","32":"WMAP <= 0","64":"ERR zero"},"binary_layout":"No header. For xy=y0*nx+x0, record offset=(xy*nz+channel0)*20. Five little-endian u32 words: SCI IEEE754 f32 bits, ERR f32 bits, physical unsigned DQ, WMAP f32 bits, derived status bits. Raw float NaN payloads and signed zero are preserved.","csv":"spaxel-major then wavelength; finite f32 values promoted exactly to f64, Rust shortest-roundtrip text. Nonfinite SCI/ERR/WMAP are empty cells, never zero. All samples and original DQ values retained; binary retains all original float bits.","spectral_handling":"No resampling, interpolation, smoothing or line fitting","flux_handling":"Surface brightness MJy/sr preserved; no Jy conversion, aperture correction or new background subtraction","uncertainty":"Input ERR retained, not a full covariance model; no claim of independent adjacent samples","extraction_image_reads":"Each of four image payloads read once in wavelength planes; bounded transpose tile of up to 16 planes. Hashing, snapshot and verification are additional passes.","excluded_inputs":["x1d.fits","stage1_spectra.csv","all Python-derived spectra"],"authentication":"CHECKSUM_ONLY_NOT_SIGNATURE_OR_PROOF_OF_CALIBRATION"}});
    let bytes = serde_json::to_vec_pretty(&receipt)?;
    write_new(&out.join("receipt.json"), &bytes)?;
    Ok(hash_bytes(&bytes))
}

// Second implementation: compare each source wavelength-plane byte sequence against
// strided output records. No forward-transpose record() function is called here.
fn verify(out: &Path, anchor: &str) -> Result<Value> {
    need(
        hash_file(&out.join("receipt.json"))? == anchor,
        "External receipt anchor mismatch",
    )?;
    let receipt: Value = serde_json::from_slice(&fs::read(out.join("receipt.json"))?)?;
    for (name, entry) in receipt["outputs"]
        .as_object()
        .ok_or("Missing output inventory")?
    {
        need(
            !name.contains('/') && !name.contains('\\') && name != "..",
            "Unsafe inventory path",
        )?;
        let p = out.join(name);
        need(
            fs::metadata(&p)?.len() == entry["bytes"].as_u64().ok_or("Bad length")?,
            "Output byte count mismatch",
        )?;
        need(
            hash_file(&p)? == entry["sha256"].as_str().ok_or("Bad hash")?,
            "Output digest mismatch",
        )?;
    }
    let input = out.join("input.fits");
    let f = FitsFile::open(&input)?;
    let l = layout(&f)?;
    need(
        f.sha256 == receipt["input_sha256"],
        "Input evidence mismatch",
    )?;
    let mut original = File::open(&input)?;
    let binary = fs::read(out.join("spectra.bin"))?;
    need(
        binary.len() == l.nx * l.ny * l.nz * RECORD,
        "Binary length mismatch",
    )?;
    let mut compared = 0usize;
    let mut scalar_checked = 0usize;
    let mut good = 0u64;
    let mut uncertainties = 0u64;
    for k in 0..l.nz {
        let p: Vec<_> = l
            .hdus
            .iter()
            .map(|h| read_plane(&mut original, h, k, l.nx * l.ny))
            .collect::<Result<_>>()?;
        for y in 0..l.ny {
            for x in 0..l.nx {
                let spatial = y * l.nx + x;
                let b = 4 * spatial;
                let offset = (spatial * l.nz + k) * RECORD;
                let words = fields(&binary[offset..offset + RECORD]);
                let s = u32::from_be_bytes(p[0][b..b + 4].try_into()?);
                let e = u32::from_be_bytes(p[1][b..b + 4].try_into()?);
                let d = u32::from_be_bytes(p[2][b..b + 4].try_into()?) ^ 0x80000000;
                let w = u32::from_be_bytes(p[3][b..b + 4].try_into()?);
                need(
                    words[..4] == [s, e, d, w],
                    "Independent full-voxel byte comparison failed",
                )?;
                let sv = f32::from_bits(s);
                let ev = f32::from_bits(e);
                let wv = f32::from_bits(w);
                let checks = [
                    !sv.is_finite(),
                    !ev.is_finite(),
                    ev.is_finite() && ev < 0.,
                    d != 0,
                    !wv.is_finite(),
                    wv.is_finite() && wv <= 0.,
                    ev == 0.,
                ];
                let mut independent_flags = 0u32;
                for (bit, condition) in checks.iter().enumerate() {
                    if *condition {
                        independent_flags |= 1 << bit;
                    }
                }
                need(
                    words[4] == independent_flags,
                    "Status reconstruction failed",
                )?;
                good += u64::from(science_ok(words[4]));
                uncertainties += u64::from(uncertainty_ok(words[4]));
                // Independent existing Goblin++ scalar decoder on reproducible spread, including endpoints.
                let flat = k * l.nx * l.ny + spatial;
                if flat % 997 == 0 || flat == l.nx * l.ny * l.nz - 1 {
                    for (j, expected) in [sv as f64, ev as f64, d as f64, wv as f64]
                        .iter()
                        .enumerate()
                    {
                        let got = f.pixel(l.hdus[j].index, flat)?;
                        need(
                            got == *expected || (got.is_nan() && expected.is_nan()),
                            "Goblin++ scalar decoder cross-check failed",
                        )?;
                        scalar_checked += 1;
                    }
                }
                compared += 1;
            }
        }
    }
    need(
        good == receipt["summary"]["science_usable_voxels"]
            .as_u64()
            .ok_or("Missing good count")?,
        "Science count mismatch",
    )?;
    need(
        uncertainties
            == receipt["summary"]["uncertainty_usable_voxels"]
                .as_u64()
                .ok_or("Missing uncertainty count")?,
        "Uncertainty count mismatch",
    )?;
    let mut lines = BufReader::new(File::open(out.join("spaxel_spectra.csv"))?).lines();
    need(
        lines.next().transpose()?.as_deref() == Some(CSV_HEADER),
        "CSV header mismatch",
    )?;
    for xy in 0..l.nx * l.ny {
        for k in 0..l.nz {
            let i = (xy * l.nz + k) * RECORD;
            let row = csv_row(
                xy % l.nx,
                xy / l.nx,
                k,
                l.wavelengths[k],
                &binary[i..i + RECORD],
            );
            need(
                lines.next().transpose()?.as_deref() == Some(row.trim_end()),
                "CSV/binary mismatch or omitted channel",
            )?;
        }
    }
    need(lines.next().is_none(), "Extra CSV rows")?;
    Ok(
        json!({"status":"PASS","receipt_sha256":anchor,"full_voxels_compared":compared,"raw_fields_compared":compared*4,"goblin_scalar_values_cross_checked":scalar_checked,"csv_rows_verified":compared,"all_channels_retained":true,"input_sha256":f.sha256,"scope":"Extraction of already calibrated cube; not independent JWST calibration, covariance estimation, or a signed provenance claim"}),
    )
}
fn spectrum(out: &Path, anchor: &str, x: usize, y: usize, dest: &Path) -> Result<()> {
    need(
        hash_file(&out.join("receipt.json"))? == anchor,
        "External receipt anchor mismatch",
    )?;
    let receipt: Value = serde_json::from_slice(&fs::read(out.join("receipt.json"))?)?;
    let dims = receipt["metadata"]["dimensions"]
        .as_array()
        .ok_or("Missing dimensions")?;
    let nx = dims[0].as_u64().ok_or("Bad nx")? as usize;
    let ny = dims[1].as_u64().ok_or("Bad ny")? as usize;
    let nz = dims[2].as_u64().ok_or("Bad nz")? as usize;
    need(x < nx && y < ny, "Spaxel coordinate out of range")?;
    need(
        hash_file(&out.join("spectra.bin"))? == receipt["outputs"]["spectra.bin"]["sha256"],
        "Binary checksum mismatch",
    )?;
    need(
        hash_file(&out.join("wavelengths.csv"))? == receipt["outputs"]["wavelengths.csv"]["sha256"],
        "Wavelength checksum mismatch",
    )?;
    let mut input = File::open(out.join("spectra.bin"))?;
    input.seek(SeekFrom::Start(((y * nx + x) * nz * RECORD) as u64))?;
    let waves: Vec<f64> = BufReader::new(File::open(out.join("wavelengths.csv"))?)
        .lines()
        .skip(1)
        .map(|line| -> Result<f64> {
            Ok(line?
                .split(',')
                .nth(1)
                .ok_or("Bad wavelength row")?
                .parse()?)
        })
        .collect::<Result<_>>()?;
    need(waves.len() == nz, "Bad wavelength table length")?;
    let mut output = BufWriter::new(new_file(dest)?);
    writeln!(output, "{CSV_HEADER}")?;
    for (k, w) in waves.iter().enumerate() {
        let mut r = [0; RECORD];
        input.read_exact(&mut r)?;
        output.write_all(csv_row(x, y, k, *w, &r).as_bytes())?;
    }
    output.flush()?;
    Ok(())
}
pub fn cli() -> Result<()> {
    let a: Vec<_> = std::env::args().collect();
    match a.get(1).map(String::as_str) {
        Some("extract") if a.len()==5=>{let sha=extract(Path::new(&a[2]),&a[3],Path::new(&a[4]))?;println!("RECEIPT_SHA256={sha}");}
        Some("verify") if a.len()==4=>{let v=verify(Path::new(&a[2]),&a[3])?;save_verification(Path::new(&a[2]),&v)?;println!("{}",serde_json::to_string_pretty(&v)?);}
        Some("spectrum") if a.len()==7=>spectrum(Path::new(&a[2]),&a[3],a[4].parse()?,a[5].parse()?,Path::new(&a[6]))?,
        _=>return Err(format!("Usage: goblin-cube-extract extract INPUT EXPECTED_SHA256 NEW_OUTPUT_DIR\n       goblin-cube-extract verify OUTPUT_DIR EXPECTED_RECEIPT_SHA256\n       goblin-cube-extract spectrum OUTPUT_DIR EXPECTED_RECEIPT_SHA256 X0 Y0 NEW_CSV\nPinned study cube: {EXPECTED}").into())
    }
    Ok(())
}

fn save_verification(out: &Path, value: &Value) -> Result<()> {
    let destination = out.join("verification.json");
    if destination.exists() {
        let previous: Value = serde_json::from_slice(&fs::read(destination)?)?;
        need(
            previous == *value,
            "Existing verification differs; refusing to overwrite it",
        )?;
    } else {
        write_new(&destination, &serde_json::to_vec_pretty(value)?)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
