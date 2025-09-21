use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use image::{ImageBuffer, Rgba};
use reqwest::blocking::Client;
use serde::Serialize;
use tiny_skia::Pixmap;
use walkdir::WalkDir;

#[derive(Parser)]
#[command(name = "xtask", about = "Dev tools for Solitaire assets")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
#[command(rename_all = "kebab-case")]
enum Cmd {
    /// Download Byron Knoll vector playing cards (Public Domain) to a temp dir
    DownloadByron { out: PathBuf },
    /// Generate a 13x4 sprite sheet from Byron SVGs, write res/cards.png, and update res/app.rc
    #[command(alias = "GenCards")]
    GenCards {
        /// Target width per card in pixels
        #[arg(long, default_value_t = 224)]
        card_w: u32,
        /// Target height per card in pixels
        #[arg(long, default_value_t = 312)]
        card_h: u32,
        /// Optional output sprite path (default: res/cards.png)
        #[arg(long)]
        out: Option<PathBuf>,
        /// Optional source directory containing SVGs (if omitted, download to temp)
        #[arg(long)]
        source: Option<PathBuf>,
        /// Update res/app.rc to embed the output
        #[arg(long, default_value_t = true)]
        update_rc: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::DownloadByron { out } => {
            download_byron(&out)?;
            println!("Downloaded to {}", out.display());
        }
        Cmd::GenCards {
            card_w,
            card_h,
            out,
            source,
            update_rc,
        } => {
            let tmp_dir;
            let src_dir = match source {
                Some(p) => p,
                None => {
                    tmp_dir = tempfile::tempdir()?;
                    let out = tmp_dir.path().to_path_buf();
                    // Try Byron first, then SVG-cards, then Kenney
                    if let Err(e) = download_byron(&out) {
                        eprintln!("Warn: Byron download failed: {e}");
                    }
                    if locate_card_source(&out).is_err() {
                        if let Err(e) = download_svgcards(&out) {
                            eprintln!("Warn: SVG-cards download failed: {e}");
                        }
                    }
                    if locate_card_source(&out).is_err() {
                        if let Err(e) = download_kenney(&out) {
                            eprintln!("Warn: Kenney download failed: {e}");
                        }
                    }
                    out
                }
            };
            let source_kind = locate_card_source(&src_dir).map_err(|_| {
                anyhow!(
                    "Could not locate per-card SVG or PNGs under {}",
                    src_dir.display()
                )
            })?;
            let out_path = out.unwrap_or_else(|| PathBuf::from("res/cards.png"));
            let map = match source_kind {
                CardSource::SvgDir(dir) => rasterize_and_pack_svg(&dir, card_w, card_h, &out_path)?,
                CardSource::PngDir(dir) => pack_from_png(&dir, card_w, card_h, &out_path)?,
            };
            if update_rc {
                update_app_rc(&PathBuf::from("res/app.rc"), &out_path)?;
            }
            // Optionally also write mapping JSON (for debugging)
            let map_path = out_path.with_extension("json");
            fs::write(&map_path, serde_json::to_vec_pretty(&map)?)?;
            println!("Sprite sheet: {}", out_path.display());
        }
    }
    Ok(())
}

fn download_byron(out: &Path) -> Result<()> {
    fs::create_dir_all(out)?;
    let url = "https://github.com/notpeter/Vector-Playing-Cards/archive/refs/heads/master.zip";
    let zip_path = out.join("byron.zip");
    let client = Client::new();
    let mut resp = client.get(url).send().context("GET repo zip")?;
    if !resp.status().is_success() {
        return Err(anyhow!("Download failed: {}", resp.status()));
    }
    let mut file = File::create(&zip_path)?;
    let mut buf = Vec::new();
    resp.copy_to(&mut buf)?;
    file.write_all(&buf)?;

    extract_zip(&zip_path, out)?;
    Ok(())
}

fn download_svgcards(out: &Path) -> Result<()> {
    fs::create_dir_all(out)?;
    let url = "https://github.com/htdebeer/SVG-cards/archive/refs/heads/master.zip";
    let zip_path = out.join("svg-cards.zip");
    let client = Client::new();
    let mut resp = client.get(url).send().context("GET svg-cards zip")?;
    if !resp.status().is_success() {
        return Err(anyhow!("Download failed: {}", resp.status()));
    }
    let mut file = File::create(&zip_path)?;
    let mut buf = Vec::new();
    resp.copy_to(&mut buf)?;
    file.write_all(&buf)?;
    extract_zip(&zip_path, out)?;
    Ok(())
}

fn download_kenney(out: &Path) -> Result<()> {
    fs::create_dir_all(out)?;
    let url = "https://github.com/kenneyNL/playing-cards-pack/archive/refs/heads/master.zip";
    let zip_path = out.join("kenney.zip");
    let client = Client::new();
    let mut resp = client.get(url).send().context("GET kenney zip")?;
    if !resp.status().is_success() {
        return Err(anyhow!("Download failed: {}", resp.status()));
    }
    let mut file = File::create(&zip_path)?;
    let mut buf = Vec::new();
    resp.copy_to(&mut buf)?;
    file.write_all(&buf)?;
    extract_zip(&zip_path, out)?;
    Ok(())
}

fn extract_zip(zip_path: &Path, out_dir: &Path) -> Result<()> {
    let mut archive = zip::ZipArchive::new(File::open(zip_path)?)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = out_dir.join(file.mangled_name());
        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p)?;
            }
            let mut outfile = File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }
    Ok(())
}

enum CardSource {
    SvgDir(PathBuf),
    PngDir(PathBuf),
}

fn locate_card_source(root: &Path) -> Result<CardSource> {
    if let Ok(svg) = find_cards_svg_dir(root) {
        return Ok(CardSource::SvgDir(svg));
    }
    if let Ok(png) = find_cards_png_dir(root) {
        return Ok(CardSource::PngDir(png));
    }
    Err(anyhow!("no card source found under {}", root.display()))
}

fn find_cards_svg_dir(root: &Path) -> Result<PathBuf> {
    // Strategy:
    // 1) Prefer a directory containing >= 50 SVGs (likely the card set root)
    // 2) Otherwise pick a directory that has any SVG file matching common patterns
    let mut best_dir: Option<(PathBuf, usize)> = None;
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_dir() {
            let dir = entry.path();
            let mut svg_count = 0usize;
            let mut has_card_like = false;
            if let Ok(rd) = fs::read_dir(dir) {
                for f in rd {
                    if let Ok(de) = f {
                        let p = de.path();
                        if p.extension()
                            .and_then(|e| e.to_str())
                            .map(|e| e.eq_ignore_ascii_case("svg"))
                            == Some(true)
                        {
                            svg_count += 1;
                            let stem = p
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("")
                                .to_lowercase();
                            if stem.contains("spade")
                                || stem.contains("heart")
                                || stem.contains("diamond")
                                || stem.contains("club")
                            {
                                has_card_like = true;
                            }
                        }
                    }
                }
            }
            if svg_count >= 50 && has_card_like {
                return Ok(dir.to_path_buf());
            }
            if has_card_like {
                if let Some((_, best_count)) = &best_dir {
                    if svg_count > *best_count {
                        best_dir = Some((dir.to_path_buf(), svg_count));
                    }
                } else {
                    best_dir = Some((dir.to_path_buf(), svg_count));
                }
            }
        }
    }
    if let Some((p, _)) = best_dir {
        return Ok(p);
    }
    Err(anyhow!(
        "Could not locate SVG card directory under {}",
        root.display()
    ))
}

fn find_cards_png_dir(root: &Path) -> Result<PathBuf> {
    let mut best_dir: Option<(PathBuf, usize)> = None;
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_dir() {
            let dir = entry.path();
            let mut png_count = 0usize;
            let mut has_card_like = false;
            if let Ok(rd) = fs::read_dir(dir) {
                for f in rd {
                    if let Ok(de) = f {
                        let p = de.path();
                        if p.extension()
                            .and_then(|e| e.to_str())
                            .map(|e| e.eq_ignore_ascii_case("png"))
                            == Some(true)
                        {
                            png_count += 1;
                            let stem = p
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("")
                                .to_lowercase();
                            if stem.contains("spade")
                                || stem.contains("heart")
                                || stem.contains("diamond")
                                || stem.contains("club")
                                || stem.contains("card")
                            {
                                has_card_like = true;
                            }
                        }
                    }
                }
            }
            if png_count >= 50 && has_card_like {
                return Ok(dir.to_path_buf());
            }
            if has_card_like {
                if let Some((_, best_count)) = &best_dir {
                    if png_count > *best_count {
                        best_dir = Some((dir.to_path_buf(), png_count));
                    }
                } else {
                    best_dir = Some((dir.to_path_buf(), png_count));
                }
            }
        }
    }
    if let Some((p, _)) = best_dir {
        return Ok(p);
    }
    Err(anyhow!(
        "Could not locate PNG card directory under {}",
        root.display()
    ))
}

fn find_svg_for(svg_dir: &Path, rank: &str, suit: &str) -> Result<PathBuf> {
    let rank = rank.to_lowercase();
    let suit = suit.to_lowercase();
    let suit_singular = suit.trim_end_matches('s');
    let candidates = [
        format!("{}_of_{}.svg", rank, suit),
        format!("{}_of_{}.svg", rank, suit_singular),
        format!("{}-of-{}.svg", rank, suit),
        format!("{}-of-{}.svg", rank, suit_singular),
        format!("{}_{}.svg", suit, rank),
        format!("{}-{}.svg", suit, rank),
    ];
    for c in &candidates {
        let p = svg_dir.join(c);
        if p.exists() {
            return Ok(p);
        }
    }
    // Fallback: scan for stems containing both rank and suit substrings
    for entry in fs::read_dir(svg_dir)? {
        let p = entry?.path();
        if p.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("svg"))
            == Some(true)
        {
            let stem = p
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            if stem.contains(&rank) && (stem.contains(&suit) || stem.contains(suit_singular)) {
                return Ok(p);
            }
        }
    }
    Err(anyhow!(
        "missing SVG for {} of {} in {}",
        rank,
        suit,
        svg_dir.display()
    ))
}

#[derive(Serialize)]
struct SheetMap {
    cols: u32,
    rows: u32,
    card_w: u32,
    card_h: u32,
    order: Vec<String>, // suits order per row
}

fn downsample_pixmap(
    pixmap: &Pixmap,
    factor: u32,
) -> anyhow::Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    anyhow::ensure!(factor > 0, "factor must be > 0");
    let src_w = pixmap.width();
    let src_h = pixmap.height();
    anyhow::ensure!(
        src_w % factor == 0,
        "width {} not divisible by {}",
        src_w,
        factor
    );
    anyhow::ensure!(
        src_h % factor == 0,
        "height {} not divisible by {}",
        src_h,
        factor
    );

    let dst_w = src_w / factor;
    let dst_h = src_h / factor;
    let mut out = vec![0u8; (dst_w * dst_h * 4) as usize];
    let samples = (factor * factor) as f32;
    let data = pixmap.data();

    for y in 0..dst_h {
        for x in 0..dst_w {
            let mut sum_r = 0.0;
            let mut sum_g = 0.0;
            let mut sum_b = 0.0;
            let mut sum_a = 0.0;
            for oy in 0..factor {
                let sy = y * factor + oy;
                let row = (sy * src_w) as usize * 4;
                for ox in 0..factor {
                    let sx = x * factor + ox;
                    let idx = row + sx as usize * 4;
                    let b = data[idx] as f32 / 255.0;
                    let g = data[idx + 1] as f32 / 255.0;
                    let r = data[idx + 2] as f32 / 255.0;
                    let a = data[idx + 3] as f32 / 255.0;
                    sum_r += r;
                    sum_g += g;
                    sum_b += b;
                    sum_a += a;
                }
            }
            let a = sum_a / samples;
            let mut r = sum_r / samples;
            let mut g = sum_g / samples;
            let mut b = sum_b / samples;

            if a > 0.0 {
                let inv = 1.0 / a;
                r = (r * inv).clamp(0.0, 1.0);
                g = (g * inv).clamp(0.0, 1.0);
                b = (b * inv).clamp(0.0, 1.0);
            } else {
                r = 0.0;
                g = 0.0;
                b = 0.0;
            }
            let dest_idx = ((y * dst_w + x) * 4) as usize;
            out[dest_idx] = (r * 255.0 + 0.5) as u8;
            out[dest_idx + 1] = (g * 255.0 + 0.5) as u8;
            out[dest_idx + 2] = (b * 255.0 + 0.5) as u8;
            out[dest_idx + 3] = (a * 255.0 + 0.5) as u8;
        }
    }

    ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(dst_w, dst_h, out)
        .ok_or_else(|| anyhow::anyhow!("downsample buffer conversion failed"))
}
fn rasterize_and_pack_svg(
    svg_dir: &Path,
    card_w: u32,
    card_h: u32,
    out_png: &Path,
) -> Result<SheetMap> {
    const SVG_OVERSAMPLE: u32 = 8;
    // Order: spades, hearts, diamonds, clubs
    let suits = ["spades", "hearts", "diamonds", "clubs"];
    let ranks = [
        "ace", "2", "3", "4", "5", "6", "7", "8", "9", "10", "jack", "queen", "king",
    ];

    let sheet_w = card_w * ranks.len() as u32;
    let sheet_h = card_h * suits.len() as u32;
    let mut sheet: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(sheet_w, sheet_h);

    for (row, suit) in suits.iter().enumerate() {
        for (col, rank) in ranks.iter().enumerate() {
            let path = find_svg_for(svg_dir, rank, suit)
                .with_context(|| format!("locating {} of {}", rank, suit))?;
            let render_w = card_w * SVG_OVERSAMPLE;
            let render_h = card_h * SVG_OVERSAMPLE;
            let pixmap = render_svg(&path, render_w, render_h)
                .with_context(|| format!("rendering {}", path.display()))?;
            let img = downsample_pixmap(&pixmap, SVG_OVERSAMPLE)?;
            image::imageops::replace(
                &mut sheet,
                &img,
                (col as u32 * card_w) as i64,
                (row as u32 * card_h) as i64,
            );
        }
    }

    if let Some(parent) = out_png.parent() {
        fs::create_dir_all(parent)?;
    }
    sheet.save(out_png)?;

    Ok(SheetMap {
        cols: ranks.len() as u32,
        rows: suits.len() as u32,
        card_w,
        card_h,
        order: suits.iter().map(|s| s.to_string()).collect(),
    })
}
fn pack_from_png(png_dir: &Path, card_w: u32, card_h: u32, out_png: &Path) -> Result<SheetMap> {
    let suits = ["spades", "hearts", "diamonds", "clubs"];
    let ranks = [
        "ace", "2", "3", "4", "5", "6", "7", "8", "9", "10", "jack", "queen", "king",
    ];
    let sheet_w = card_w * ranks.len() as u32;
    let sheet_h = card_h * suits.len() as u32;
    let mut sheet: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(sheet_w, sheet_h);

    for (row, suit) in suits.iter().enumerate() {
        for (col, rank) in ranks.iter().enumerate() {
            let path = find_png_for(png_dir, rank, suit)
                .with_context(|| format!("locating {} of {}", rank, suit))?;
            let img_dyn = image::open(&path)?; // supports many PNG variants
            let img = img_dyn
                .resize_exact(card_w, card_h, image::imageops::FilterType::CatmullRom)
                .to_rgba8();
            image::imageops::replace(
                &mut sheet,
                &img,
                (col as u32 * card_w) as i64,
                (row as u32 * card_h) as i64,
            );
        }
    }

    if let Some(parent) = out_png.parent() {
        fs::create_dir_all(parent)?;
    }
    sheet.save(out_png)?;
    Ok(SheetMap {
        cols: ranks.len() as u32,
        rows: suits.len() as u32,
        card_w,
        card_h,
        order: suits.iter().map(|s| s.to_string()).collect(),
    })
}

fn find_png_for(png_dir: &Path, rank: &str, suit: &str) -> Result<PathBuf> {
    let rank_l = rank.to_lowercase();
    let suit_l = suit.to_lowercase();
    let suit_cap = capitalize(&suit_l);
    let rank_letter = match rank_l.as_str() {
        "ace" => "A",
        "jack" => "J",
        "queen" => "Q",
        "king" => "K",
        other => other, // "2".."10"
    };
    let ten_letter = if rank_l == "10" { "T" } else { rank_letter };
    let suit_letter = match suit_l.as_str() {
        "spades" => "S",
        "hearts" => "H",
        "diamonds" => "D",
        "clubs" => "C",
        _ => "",
    };

    let mut candidates = Vec::new();
    // Common long forms
    candidates.push(format!("{}_of_{}.png", rank_l, suit_l));
    candidates.push(format!("{}-of-{}.png", rank_l, suit_l));
    candidates.push(format!("{}_{}.png", rank_l, suit_l));
    candidates.push(format!("{}-{}.png", rank_l, suit_l));
    candidates.push(format!("{}_{}.png", suit_l, rank_l));
    candidates.push(format!("{}-{}.png", suit_l, rank_l));
    // Shorthand like AS.png, TS.png (or 10S.png)
    candidates.push(format!("{}{}.png", rank_letter, suit_letter));
    candidates.push(format!("{}{}.png", ten_letter, suit_letter));
    // Kenney styles
    candidates.push(format!("card{}{}.png", suit_cap, rank_letter));
    candidates.push(format!("{}{}_card.png", rank_letter, suit_cap));
    candidates.push(format!("{}{}_card.png", suit_cap, rank_letter));

    for c in &candidates {
        let p = png_dir.join(c);
        if p.exists() {
            return Ok(p);
        }
        // Try case variations
        let lower = png_dir.join(c.to_lowercase());
        if lower.exists() {
            return Ok(lower);
        }
        let upper = png_dir.join(c.to_uppercase());
        if upper.exists() {
            return Ok(upper);
        }
    }
    // Fallback: recursive scan under png_dir for any file containing both suit and rank tokens
    let suit_singular = suit_l.trim_end_matches('s');
    let mut suit_terms = vec![suit_l.as_str()];
    if suit_singular != suit_l {
        suit_terms.push(suit_singular);
    }

    let mut rank_terms = vec![rank_l.as_str()];
    if rank_l == "10" {
        rank_terms.push("ten");
    }
    match rank_l.as_str() {
        "ace" => rank_terms.extend(["1", "01"]),
        "jack" => rank_terms.push("11"),
        "queen" => rank_terms.push("12"),
        "king" => rank_terms.push("13"),
        _ => {}
    }

    for entry in WalkDir::new(png_dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let p = entry.path();
            if p.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("png"))
                == Some(true)
            {
                let stem = p
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                let has_suit = suit_terms
                    .iter()
                    .any(|term| !term.is_empty() && stem.contains(term));
                let has_rank = rank_terms
                    .iter()
                    .any(|term| !term.is_empty() && stem.contains(term));
                if has_suit && has_rank {
                    return Ok(p.to_path_buf());
                }
            }
        }
    }
    Err(anyhow!(
        "missing PNG for {} of {} in {}",
        rank,
        suit,
        png_dir.display()
    ))
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn render_svg(path: &Path, w: u32, h: u32) -> Result<Pixmap> {
    let mut data = Vec::new();
    File::open(path)?.read_to_end(&mut data)?;
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(&data, &opt).map_err(|e| anyhow!("usvg parse: {:?}", e))?;

    // Fit to requested size while preserving aspect
    let size = tree.size();
    let width = size.width();
    let height = size.height();
    let scale_x = w as f32 / width;
    let scale_y = h as f32 / height;
    let scale = scale_x.min(scale_y);
    let target_w = (width * scale).round() as u32;
    let target_h = (height * scale).round() as u32;

    let mut pixmap = Pixmap::new(w, h).ok_or_else(|| anyhow!("pixmap alloc failed"))?;
    pixmap.fill(tiny_skia::Color::TRANSPARENT);

    let tx = ((w - target_w) as f32) * 0.5;
    let ty = ((h - target_h) as f32) * 0.5;
    let transform = tiny_skia::Transform::from_row(scale, 0.0, 0.0, scale, tx, ty);
    let mut pm = pixmap.as_mut();
    resvg::render(&tree, transform, &mut pm);

    Ok(pixmap)
}

fn update_app_rc(app_rc: &Path, png_path: &Path) -> Result<()> {
    let mut text = fs::read_to_string(app_rc)?;
    let line = format!("IDB_CARDS RCDATA \"{}\"", normalize_path_for_rc(png_path));
    if text.contains("IDB_CARDS RCDATA") {
        // Uncomment if commented
        text = text
            .lines()
            .map(|l| {
                if l.trim_start().starts_with("//") && l.contains("IDB_CARDS RCDATA") {
                    l.trim_start_matches('/')
                        .trim_start_matches('/')
                        .trim_start()
                        .to_string()
                } else {
                    l.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
    } else {
        // Append at end
        text.push_str("\n");
        text.push_str(&line);
        text.push_str("\n");
    }
    fs::write(app_rc, text)?;
    Ok(())
}

fn normalize_path_for_rc(p: &Path) -> String {
    p.to_string_lossy().replace("\\", "/")
}
