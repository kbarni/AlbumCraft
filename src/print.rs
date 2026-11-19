//! Printing: the rendered pages go to the printer as a PDF (core's `print::pdf_of_pages`, one
//! JPEG per page on the printable area), through the suite's `craft-print` on Windows and
//! through CUPS on Linux and macOS (`print_cups.rs`: IPP straight to its socket with `craft-ipp`,
//! else `craft-print`'s `lp`). The layouts and page rendering are `albumcraft_core::print`
//! (re-exported here).

pub use albumcraft_core::print::*;
use image::RgbaImage;

#[cfg(not(windows))]
#[path = "print_cups.rs"]
mod cups;

/// Whether printing works here: always on Windows; on Linux and macOS when a CUPS print service
/// answers. Without it the print commands are hidden.
pub fn available() -> bool {
    #[cfg(windows)]
    {
        true
    }
    #[cfg(not(windows))]
    {
        cups::available()
    }
}

/// A chosen printer and its settings: on Windows, after "Printer Setup", what was chosen in the
/// driver's own dialog; with CUPS, the chosen paper (None = the printer's default).
#[derive(Clone, Debug, Default)]
pub struct PrinterChoice {
    pub name: String,
    #[cfg_attr(not(windows), allow(dead_code))] // only Windows printing reads it
    pub settings: Option<craft_print::DriverSettings>,
    #[cfg_attr(windows, allow(dead_code))] // only CUPS printing reads it
    pub media: Option<String>,
}

/// What a paper name of the printer is called in the print screen: CUPS's own names (`A4`,
/// `4x6`) as they are, the IPP names (`iso_a4_210x297mm`, `na_letter_8.5x11in`) shortened to
/// `A4`, `Letter`, or the size (`4 x 6 in`).
pub fn paper_label(name: &str) -> String {
    let parts: Vec<&str> = name.split('_').collect();
    let (Some(first), Some(last)) = (parts.first(), parts.last()) else { return name.to_string() };
    let dims = last.strip_suffix("mm").map(|d| (d, "mm")).or_else(|| last.strip_suffix("in").map(|d| (d, "in")));
    let Some((dims, unit)) = dims.filter(|(d, _)| parts.len() >= 3 && d.split_once('x').is_some_and(|(w, h)| w.parse::<f32>().is_ok() && h.parse::<f32>().is_ok()))
    else {
        return name.to_string();
    };
    let size = parts[1..parts.len() - 1].join("_");
    let is_series = |s: &str| {
        let mut c = s.chars();
        matches!(c.next(), Some('a' | 'b')) && !c.as_str().is_empty() && c.all(|d| d.is_ascii_digit())
    };
    if is_series(&size) {
        let series = size.to_ascii_uppercase();
        return if *first == "jis" { format!("JIS {series}") } else { series };
    }
    if matches!(size.as_str(), "letter" | "legal" | "executive" | "ledger" | "tabloid") {
        let mut c = size.chars();
        return c.next().map(|f| f.to_ascii_uppercase().to_string() + c.as_str()).unwrap_or_default();
    }
    let (w, h) = dims.split_once('x').unwrap_or((dims, ""));
    format!("{w} x {h} {unit}")
}

/// Print the pages (each covering the printable area), or write the PDF to `to_file`.
pub fn print_pages(p: &PrinterChoice, doc: &str, pages: &mut dyn Iterator<Item = RgbaImage>, to_file: Option<&str>) -> Result<usize, String> {
    let paper = paper(p).unwrap_or_else(Paper::a4);
    let mut jpegs = vec![];
    for page in pages {
        let rgb = image::DynamicImage::ImageRgba8(page).to_rgb8();
        let mut buf = vec![];
        rgb.write_with_encoder(image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 92)).map_err(|e| e.to_string())?;
        jpegs.push((buf, rgb.width(), rgb.height()));
    }
    if jpegs.is_empty() {
        return Err("nothing to print".into());
    }
    let pdf = pdf_of_pages(&jpegs, &paper);
    if let Some(path) = to_file {
        std::fs::write(path, pdf).map_err(|e| format!("{path}: {e}"))?;
        return Ok(jpegs.len());
    }
    if p.name.is_empty() {
        return Err("no printer chosen".into());
    }
    let title: String = doc.chars().filter(|c| !c.is_control()).take(100).collect();
    submit(p, &title, &pdf)?;
    Ok(jpegs.len())
}

/// Printing on Windows through `craft-print`: the printers from the spooler, the paper and
/// printable area from the driver, the driver's own dialog for its settings, and the PDF drawn
/// by Windows' own PDF renderer into the XPS print API, all in this process.
#[cfg(windows)]
mod win {
    use super::*;
    use craft_print::rgb;

    pub fn printers() -> Vec<String> {
        // the queues without asking each driver, so a printer in power save can't hold the list
        craft_print::printer_queues().into_iter().map(|p| p.name).filter(|n| !n.is_empty()).collect()
    }

    pub fn default_printer() -> Option<String> {
        rgb::default_printer().ok().flatten().filter(|n| !n.is_empty())
    }

    /// The driver settings chosen for this printer (settings chosen for another are left out).
    fn settings(p: &PrinterChoice) -> Option<&rgb::DevMode> {
        p.settings.as_ref().filter(|s| s.printer() == p.name).map(|s| s.dev_mode())
    }

    pub fn paper(p: &PrinterChoice) -> Option<Paper> {
        if p.name.is_empty() {
            return None;
        }
        let page = rgb::page(&p.name, settings(p)).ok()?;
        let mm = |pt: f64| (pt / 72.0 * 25.4) as f32;
        let (x, y, w, h) = page.printable;
        let paper = Paper {
            sheet_w: mm(page.paper.0),
            sheet_h: mm(page.paper.1),
            area_x: mm(x),
            area_y: mm(y),
            area_w: mm(w),
            area_h: mm(h),
            dpi: page.dpi.0.clamp(72, 2400),
        };
        (paper.area_w > 10.0 && paper.area_h > 10.0).then_some(paper)
    }

    /// The printer driver's own settings dialog (paper size, orientation, quality…), for the
    /// pictures printed from this screen.
    pub fn setup_dialog(current: &PrinterChoice) -> Option<PrinterChoice> {
        let mut job = craft_print::Job {
            printer: (!current.name.is_empty()).then(|| current.name.clone()),
            driver_settings: current.settings.clone(),
            ..Default::default()
        };
        match craft_print::driver_settings_dialog(&mut job, None) {
            Ok(true) => {
                let settings = job.driver_settings;
                let name = settings.as_ref().map(|s| s.printer().to_string()).unwrap_or_else(|| current.name.clone());
                Some(PrinterChoice { name, settings, media: None })
            }
            _ => None,
        }
    }

    pub fn submit(p: &PrinterChoice, title: &str, pdf: &[u8]) -> Result<(), String> {
        // copies are already pages of the PDF; the sheet is the driver's paper, so no scaling
        let job = craft_print::Job {
            printer: Some(p.name.clone()),
            title: title.to_string(),
            scaling: craft_print::Scaling::Exact,
            driver_settings: p.settings.clone(),
            ..Default::default()
        };
        craft_print::submit(pdf, &job).map(|_| ()).map_err(|e| e.to_string())
    }
}

#[cfg(windows)]
use win::submit;
#[cfg(windows)]
pub use win::{default_printer, paper, printers, setup_dialog};

/// Paper sizes to choose from in the print screen (CUPS); empty on Windows, where the printer's
/// own setup dialog does that.
#[cfg(windows)]
pub fn paper_sizes(_: &str) -> Vec<String> {
    vec![]
}

#[cfg(not(windows))]
use cups::submit;
#[cfg(not(windows))]
pub use cups::{default_printer, paper, paper_sizes, printers, setup_dialog};

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn paper_names_shown() {
        assert_eq!(paper_label("iso_a4_210x297mm"), "A4");
        assert_eq!(paper_label("iso_a5_148x210mm"), "A5");
        assert_eq!(paper_label("jis_b5_182x257mm"), "JIS B5");
        assert_eq!(paper_label("na_letter_8.5x11in"), "Letter");
        assert_eq!(paper_label("na_index-4x6_4x6in"), "4 x 6 in");
        assert_eq!(paper_label("om_small-photo_100x150mm"), "100 x 150 mm");
        assert_eq!(paper_label("custom_148.52x209.9mm_148.52x209.9mm"), "148.52 x 209.9 mm");
        // CUPS's own names stay as they are
        assert_eq!(paper_label("A4"), "A4");
        assert_eq!(paper_label("4x6.Borderless"), "4x6.Borderless");
        assert_eq!(paper_label("w288h432"), "w288h432");
    }

    #[test]
    fn writes_a_pdf_to_a_file_without_printing() {
        // `to_file` writes the PDF instead of sending it, so this never reaches a real printer
        let page = RgbaImage::from_pixel(200, 287, Rgba([200, 30, 30, 255]));
        let out = std::env::temp_dir().join(format!("albumcraft-print-{}.pdf", std::process::id()));
        let choice = PrinterChoice { name: String::new(), ..Default::default() };
        let n = print_pages(&choice, "test", &mut vec![page.clone(), page].into_iter(), Some(&out.to_string_lossy())).unwrap();
        let pdf = std::fs::read(&out).unwrap();
        let _ = std::fs::remove_file(&out);
        assert_eq!(n, 2);
        let text = String::from_utf8_lossy(&pdf);
        assert!(pdf.starts_with(b"%PDF-1.4") && text.contains("/Count 2") && text.trim_end().ends_with("%%EOF"));
        assert!(text.contains("/MediaBox [0 0 595.28 841.89]"), "A4 in points");
        // the cross-reference table points at the objects
        let xref: usize = text.rsplit("startxref").next().unwrap().split_whitespace().next().unwrap().parse().unwrap();
        assert!(pdf[xref..].starts_with(b"xref"));
    }

    #[test]
    fn no_printer_chosen_is_an_error() {
        let page = RgbaImage::from_pixel(20, 28, Rgba([200, 30, 30, 255]));
        assert!(print_pages(&PrinterChoice::default(), "test", &mut std::iter::once(page), None).is_err());
        assert!(print_pages(&PrinterChoice::default(), "test", &mut std::iter::empty(), None).is_err());
    }

    /// `ALBUMCRAFT_PDF_OUT=out.pdf cargo test print_pdf_demo -- --ignored`: a two-page sample PDF
    /// (A4, one 10 x 15 layout page) to open in a viewer; nothing is printed.
    #[test]
    #[ignore]
    fn print_pdf_demo() {
        let out = std::env::var("ALBUMCRAFT_PDF_OUT").expect("set ALBUMCRAFT_PDF_OUT");
        let paper = Paper::a4();
        let cells = page_cells(&paper, PrintLayout::Cm10x15);
        let img = crate::imaging::sample_image(4, 1200, 900);
        let items: Vec<PageItem> = (0..cells.len()).map(|i| PageItem { image: img.clone(), caption: format!("Sample {i}") }).collect();
        let settings = PrintSettings { caption: CaptionMode::FileName, border: BorderStyle::ThinBlack, ..Default::default() };
        let page = render_page(&paper, 150, &cells, &items, &settings);
        let choice = PrinterChoice::default();
        assert_eq!(print_pages(&choice, "demo", &mut vec![page.clone(), page].into_iter(), Some(&out)).unwrap(), 2);
    }

    /// The paper "Microsoft Print to PDF" reports, and a PDF of one page laid out on it (written
    /// to a file, not printed). `cargo test windows_paper -- --ignored --nocapture`.
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn windows_paper() {
        let choice = PrinterChoice { name: "Microsoft Print to PDF".into(), ..Default::default() };
        let paper = paper(&choice).expect("printer");
        println!("{paper:?}");
        assert!(paper.sheet_w > 100.0 && paper.sheet_h > 100.0);
        let cells = page_cells(&paper, PrintLayout::Cm10x15);
        let img = crate::imaging::sample_image(4, 1200, 900);
        let items: Vec<PageItem> = (0..cells.len()).map(|i| PageItem { image: img.clone(), caption: format!("Sample {i}") }).collect();
        let page = render_page(&paper, 150, &cells, &items, &PrintSettings::default());
        let out = std::env::temp_dir().join("albumcraft-print-test.pdf");
        assert_eq!(print_pages(&choice, "AlbumCraft test", &mut std::iter::once(page), Some(out.to_str().unwrap())).unwrap(), 1);
        assert!(out.exists(), "no PDF written");
    }
}
