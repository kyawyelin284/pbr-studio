//! Report export to HTML and PDF formats.
//!
//! Supports single and batch report generation with material scores,
//! issues, suggestions, and optimization actions.

use crate::json_report::{MaterialReport, Severity};
use std::path::Path;
use std::fs;

/// HTML export for a single report
pub fn export_html_single(report: &MaterialReport, output_path: &Path) -> Result<(), crate::Error> {
    let html = render_html_single(report);
    fs::write(output_path, html)?;
    Ok(())
}

/// HTML export for batch reports (multiple materials)
pub fn export_html_batch(
    reports: &[(String, MaterialReport)],
    output_path: &Path,
) -> Result<(), crate::Error> {
    let html = render_html_batch(reports);
    fs::write(output_path, html)?;
    Ok(())
}

/// PDF export for a single report (requires `pdf` feature)
#[cfg(feature = "pdf")]
pub fn export_pdf_single(report: &MaterialReport, output_path: &Path) -> Result<(), crate::Error> {
    use genpdf::elements::Paragraph;
    use genpdf::style;
    use genpdf::{Document, Margins, SimplePageDecorator};

    let font_family = load_pdf_font()?;
    let mut doc = Document::new(font_family);
    doc.set_title(report.name.as_deref().unwrap_or("PBR Material Report"));
    doc.set_minimal_conformance();
    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(Margins::all(10));
    doc.set_page_decorator(decorator);

    let name = report.name.as_deref().unwrap_or("Unknown");
    doc.push(Paragraph::default().styled_string(name, style::Style::new().with_font_size(18)));
    doc.push(Paragraph::new(format!("Score: {} / 100", report.score)));
    doc.push(Paragraph::new(format!("Status: {}", if report.passed { "Passed" } else { "Needs attention" })));
    if let Some(ref v) = report.vram_estimate {
        doc.push(Paragraph::new(format!("VRAM estimate: {}", v.formatted)));
    }

    doc.push(Paragraph::default().styled_string("Issues", style::Style::new().with_font_size(14)));
    for issue in &report.issues {
        doc.push(
            Paragraph::default().styled_string(
                format!("[{}] {}: {}", severity_str(issue.severity), issue.rule_id, issue.message),
                style::Style::new().with_font_size(9),
            ),
        );
    }

    doc.push(Paragraph::default().styled_string("Suggested optimizations", style::Style::new().with_font_size(14)));
    for s in &report.optimization_suggestions {
        doc.push(
            Paragraph::default().styled_string(
                format!("- [{}] {}", s.category, s.message),
                style::Style::new().with_font_size(9),
            ),
        );
    }

    doc.render_to_file(output_path).map_err(|e| crate::Error::Other(format!("PDF render failed: {}", e)))?;
    Ok(())
}

/// PDF export for batch reports (requires `pdf` feature)
#[cfg(feature = "pdf")]
pub fn export_pdf_batch(
    reports: &[(String, MaterialReport)],
    output_path: &Path,
) -> Result<(), crate::Error> {
    use genpdf::elements::Paragraph;
    use genpdf::style;
    use genpdf::{Document, Margins, SimplePageDecorator};

    let font_family = load_pdf_font()?;
    let mut doc = Document::new(font_family);
    doc.set_title("PBR Material Batch Report");
    doc.set_minimal_conformance();
    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(Margins::all(10));
    doc.set_page_decorator(decorator);

    doc.push(
        Paragraph::default().styled_string(
            format!("Batch Report - {} materials", reports.len()),
            style::Style::new().with_font_size(18),
        ),
    );
    doc.push(Paragraph::new(""));

    for (path, report) in reports {
        let name = report.name.as_deref().unwrap_or(path.as_str());
        doc.push(Paragraph::default().styled_string(name, style::Style::new().with_font_size(14)));
        doc.push(
            Paragraph::default().styled_string(
                format!("  Path: {}", path),
                style::Style::new().with_font_size(8),
            ),
        );
        doc.push(Paragraph::new(format!(
            "  Score: {} | Status: {}",
            report.score,
            if report.passed { "Passed" } else { "Needs attention" }
        )));
        for issue in &report.issues {
            doc.push(
                Paragraph::default().styled_string(
                    format!("    - [{}] {}", issue.rule_id, issue.message),
                    style::Style::new().with_font_size(8),
                ),
            );
        }
        doc.push(Paragraph::new(""));
    }

    doc.render_to_file(output_path).map_err(|e| crate::Error::Other(format!("PDF render failed: {}", e)))?;
    Ok(())
}

/// Bundled DejaVu Sans (SIL Open Font License). Used when system fonts are unavailable.
#[cfg(feature = "pdf")]
const BUNDLED_FONT: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");

/// Loads a font family for PDF export. Tries system fonts first (Linux, Windows, macOS),
/// then falls back to the bundled DejaVu Sans. Fully offline.
#[cfg(feature = "pdf")]
fn load_pdf_font() -> Result<genpdf::fonts::FontFamily<genpdf::fonts::FontData>, crate::Error> {
    use genpdf::fonts::{FontData, FontFamily, from_files};

    // 1. Try system font directories (platform-specific)
    if let Some(dir) = system_font_dir() {
        // LiberationSans naming: LiberationSans-Regular.ttf, etc.
        if let Ok(family) = from_files(&dir, "LiberationSans", None) {
            return Ok(family);
        }
        // DejaVu naming: DejaVuSans.ttf or DejaVuSans-Regular.ttf
        if dir.join("DejaVuSans-Regular.ttf").exists() {
            if let Ok(family) = from_files(&dir, "DejaVuSans", None) {
                return Ok(family);
            }
        }
        // macOS: Arial.ttf, Helvetica.ttf in Supplemental
        #[cfg(target_os = "macos")]
        for name in &["Arial.ttf", "Helvetica.ttf", "DejaVuSans.ttf"] {
            let p = dir.join(name);
            if p.exists() {
                if let Ok(data) = std::fs::read(&p) {
                    if let Ok(fd) = FontData::new(data, None) {
                        return Ok(FontFamily {
                            regular: fd.clone(),
                            bold: fd.clone(),
                            italic: fd.clone(),
                            bold_italic: fd.clone(),
                        });
                    }
                }
            }
        }
        if dir.join("DejaVuSans.ttf").exists() {
            // DejaVu uses DejaVuSans.ttf for regular; genpdf expects -Regular suffix.
            if let Ok(data) = std::fs::read(dir.join("DejaVuSans.ttf")) {
                if let Ok(fd) = FontData::new(data, None) {
                    return Ok(FontFamily {
                        regular: fd.clone(),
                        bold: fd.clone(),
                        italic: fd.clone(),
                        bold_italic: fd.clone(),
                    });
                }
            }
        }
        // Windows: arial.ttf, verdana.ttf etc. (different naming than Liberation/DejaVu)
        #[cfg(target_os = "windows")]
        for name in &["arial.ttf", "verdana.ttf", "tahoma.ttf"] {
            let p = dir.join(name);
            if p.exists() {
                if let Ok(data) = std::fs::read(&p) {
                    if let Ok(fd) = FontData::new(data, None) {
                        return Ok(FontFamily {
                            regular: fd.clone(),
                            bold: fd.clone(),
                            italic: fd.clone(),
                            bold_italic: fd.clone(),
                        });
                    }
                }
            }
        }
    }

    // 2. Fall back to bundled font (always works, fully offline)
    let fd = FontData::new(BUNDLED_FONT.to_vec(), None)
        .map_err(|e| crate::Error::Other(format!("Bundled font load failed: {}", e)))?;
    Ok(FontFamily {
        regular: fd.clone(),
        bold: fd.clone(),
        italic: fd.clone(),
        bold_italic: fd.clone(),
    })
}

/// Returns a directory containing usable fonts, or None. Platform-specific paths.
#[cfg(feature = "pdf")]
fn system_font_dir() -> Option<std::path::PathBuf> {
    let candidates: Vec<std::path::PathBuf> = if cfg!(target_os = "windows") {
        let mut paths = vec![std::path::PathBuf::from("C:\\Windows\\Fonts")];
        if let Ok(windir) = std::env::var("WINDIR") {
            paths.push(std::path::PathBuf::from(windir).join("Fonts"));
        }
        paths
    } else if cfg!(target_os = "macos") {
        let mut paths = vec![
            std::path::PathBuf::from("/System/Library/Fonts/Supplemental"),
            std::path::PathBuf::from("/Library/Fonts"),
            std::path::PathBuf::from("/System/Library/Fonts"),
        ];
        if let Ok(home) = std::env::var("HOME") {
            paths.push(std::path::PathBuf::from(home).join("Library/Fonts"));
        }
        paths
    } else {
        // Linux and other Unix
        vec![
            std::path::PathBuf::from("/usr/share/fonts/truetype/liberation"),
            std::path::PathBuf::from("/usr/share/fonts/TTF"),
            std::path::PathBuf::from("/usr/share/fonts/truetype/dejavu"),
            std::path::PathBuf::from("/usr/local/share/fonts/liberation"),
            std::path::PathBuf::from("/usr/share/fonts"),
        ]
    };

    for path in &candidates {
        if path.is_dir() {
            if path.join("LiberationSans-Regular.ttf").exists()
                || path.join("DejaVuSans.ttf").exists()
                || path.join("DejaVuSans-Regular.ttf").exists()
            {
                return Some(path.clone());
            }
            #[cfg(target_os = "windows")]
            if path.join("arial.ttf").exists() || path.join("verdana.ttf").exists() {
                return Some(path.clone());
            }
            #[cfg(target_os = "macos")]
            if path.join("Arial.ttf").exists()
                || path.join("Helvetica.ttf").exists()
                || path.join("DejaVuSans.ttf").exists()
            {
                return Some(path.clone());
            }
        }
    }
    None
}

#[cfg(feature = "pdf")]
fn severity_str(s: Severity) -> &'static str {
    match s {
        Severity::Critical => "CRITICAL",
        Severity::Major => "MAJOR",
        Severity::Minor => "MINOR",
    }
}

#[cfg(not(feature = "pdf"))]
pub fn export_pdf_single(_report: &MaterialReport, _output_path: &Path) -> Result<(), crate::Error> {
    Err(crate::Error::Other(
        "PDF export requires the 'pdf' feature. Build with: cargo build --features pdf".into(),
    ))
}

#[cfg(not(feature = "pdf"))]
pub fn export_pdf_batch(
    _reports: &[(String, MaterialReport)],
    _output_path: &Path,
) -> Result<(), crate::Error> {
    Err(crate::Error::Other(
        "PDF export requires the 'pdf' feature. Build with: cargo build --features pdf".into(),
    ))
}

fn render_html_single(report: &MaterialReport) -> String {
    let name = report.name.as_deref().unwrap_or("Unknown");
    let status_class = if report.passed { "passed" } else { "failed" };

    let issues_html: String = report.issues.iter()
        .map(|i| format!(
            r#"<li class="issue severity-{}"><span class="rule">{}</span> {}</li>"#,
            severity_class(i.severity),
            html_escape(&i.rule_id),
            html_escape(&i.message)
        ))
        .collect();

    let suggestions_html: String = report.optimization_suggestions.iter()
        .map(|s| {
            let details = s.details.as_ref()
                .map(|d| format!(r#"<div class="details">{}</div>"#, html_escape(d)))
                .unwrap_or_default();
            format!(
                r#"<li class="suggestion"><span class="category">{}</span> {} {}</li>"#,
                html_escape(&s.category),
                html_escape(&s.message),
                details
            )
        })
        .collect();

    let vram_html = report.vram_estimate.as_ref()
        .map(|v| format!(
            r#"<div class="vram">VRAM: {} | Packed ORM: {}</div>"#,
            html_escape(&v.formatted),
            v.packed_orm
        ))
        .unwrap_or_default();

    let ai_html = report.ai_insights.as_ref().map(|ai| {
        let mut parts = Vec::new();
        if let Some(ref c) = ai.classification {
            let conf = ai.classification_confidence.map(|f| format!(" ({:.0}%)", f * 100.0)).unwrap_or_default();
            parts.push(format!(r#"<div class="ai-class">Classification: {} {}</div>"#, html_escape(c), conf));
        }
        if let Some(ref anom) = ai.anomalies {
            if !anom.is_empty() {
                let list: String = anom.iter()
                    .map(|a| format!(r#"<li>{}: {}</li>"#, html_escape(&a.slot), html_escape(&a.message)))
                    .collect();
                parts.push(format!(r#"<div class="ai-anomalies"><strong>Anomalies:</strong><ul>{}</ul></div>"#, list));
            }
        }
        if parts.is_empty() {
            String::new()
        } else {
            format!(r#"<div class="ai-insights"><strong>AI Insights</strong>{}</div>"#, parts.join(""))
        }
    }).unwrap_or_default();

    let summary_html = report.summary.dimensions.as_ref()
        .map(|d| format!(r#"<div class="summary">{} textures | {}x{} | Maps: albedo={} normal={} roughness={} metallic={} ao={} height={}</div>"#,
            report.summary.texture_count,
            d.width, d.height,
            report.summary.maps.albedo,
            report.summary.maps.normal,
            report.summary.maps.roughness,
            report.summary.maps.metallic,
            report.summary.maps.ao,
            report.summary.maps.height,
        ))
        .unwrap_or_else(|| format!(
            r#"<div class="summary">{} textures | Maps: albedo={} normal={} roughness={} metallic={} ao={} height={}</div>"#,
            report.summary.texture_count,
            report.summary.maps.albedo,
            report.summary.maps.normal,
            report.summary.maps.roughness,
            report.summary.maps.metallic,
            report.summary.maps.ao,
            report.summary.maps.height,
        ));

    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8"/>
<meta name="viewport" content="width=device-width,initial-scale=1"/>
<title>PBR Report - {}</title>
<style>
body {{ font-family: system-ui, sans-serif; margin: 2rem; max-width: 800px; }}
h1 {{ font-size: 1.5rem; }}
.score {{ font-size: 2rem; font-weight: bold; }}
.score.passed {{ color: #198754; }}
.score.failed {{ color: #dc3545; }}
.section {{ margin: 1rem 0; }}
.section-title {{ font-weight: bold; margin-bottom: 0.5rem; }}
.issue-list, .suggestion-list {{ list-style: none; padding: 0; }}
.issue {{ padding: 0.25rem 0; }}
.severity-critical {{ color: #dc3545; }}
.severity-major {{ color: #fd7e14; }}
.severity-minor {{ color: #6c757d; }}
.suggestion {{ padding: 0.25rem 0; }}
.category {{ font-weight: 600; color: #0d6efd; }}
.details {{ font-size: 0.9em; color: #6c757d; margin-top: 0.5rem; }}
.vram {{ font-size: 0.9em; color: #6c757d; }}
.summary {{ font-size: 0.9em; color: #6c757d; }}
.ai-insights {{ font-size: 0.9em; margin-top: 0.5rem; padding: 0.5rem; background: #f8f9fa; border-radius: 8px; }}
.ai-class {{ color: #0d6efd; }}
.ai-anomalies ul {{ margin: 0.25rem 0; padding-left: 1.25rem; }}
footer {{ margin-top: 2rem; font-size: 0.8em; color: #6c757d; }}
</style>
</head>
<body>
<header>
<h1>{}</h1>
<div class="score {}">Score: {} / 100</div>
<div>Status: {}</div>
{}
{}
{}
</header>
<div class="section">
<div class="section-title">Issues</div>
<ul class="issue-list">{}</ul>
</div>
<div class="section">
<div class="section-title">Suggested Optimizations</div>
<ul class="suggestion-list">{}</ul>
</div>
<footer>Generated by PBR Studio — {}</footer>
</body>
</html>"#,
        html_escape(name),
        html_escape(name),
        status_class,
        report.score,
        if report.passed { "Passed" } else { "Needs attention" },
        vram_html,
        summary_html,
        ai_html,
        issues_html,
        suggestions_html,
        chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")
    )
}

fn render_html_batch(reports: &[(String, MaterialReport)]) -> String {
    let items: String = reports.iter()
        .map(|(path, report)| {
            let name = report.name.as_deref().unwrap_or(path.as_str());
            let status_class = if report.passed { "passed" } else { "failed" };
            let issues_html: String = report.issues.iter()
                .map(|i| format!(
                    r#"<li class="severity-{}">{}: {}</li>"#,
                    severity_class(i.severity),
                    html_escape(&i.rule_id),
                    html_escape(&i.message)
                ))
                .collect();
            let suggestions_html: String = report.optimization_suggestions.iter()
                .map(|s| format!(r#"<li>{}: {}</li>"#, html_escape(&s.category), html_escape(&s.message)))
                .collect();
            format!(
                r#"<div class="material-block">
<h2><a href="file://{}">{}</a></h2>
<div class="path">{}</div>
<div class="score {}">Score: {} / 100</div>
<div class="section"><strong>Issues</strong><ul>{}</ul></div>
<div class="section"><strong>Optimizations</strong><ul>{}</ul></div>
</div>"#,
                html_escape(path),
                html_escape(name),
                html_escape(path),
                status_class,
                report.score,
                issues_html,
                suggestions_html
            )
        })
        .collect();

    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8"/>
<meta name="viewport" content="width=device-width,initial-scale=1"/>
<title>PBR Batch Report</title>
<style>
body {{ font-family: system-ui, sans-serif; margin: 2rem; max-width: 900px; }}
h1 {{ font-size: 1.5rem; }}
.material-block {{ margin: 2rem 0; padding: 1rem; border: 1px solid #dee2e6; border-radius: 8px; }}
.material-block h2 {{ font-size: 1.1rem; margin: 0 0 0.5rem; }}
.path {{ font-size: 0.9em; color: #6c757d; margin-bottom: 0.5rem; }}
.score {{ font-weight: bold; }}
.score.passed {{ color: #198754; }}
.score.failed {{ color: #dc3545; }}
.section {{ margin: 0.5rem 0; font-size: 0.95em; }}
.section ul {{ margin: 0.25rem 0; padding-left: 1.25rem; }}
.severity-critical {{ color: #dc3545; }}
.severity-major {{ color: #fd7e14; }}
.severity-minor {{ color: #6c757d; }}
footer {{ margin-top: 2rem; font-size: 0.8em; color: #6c757d; }}
</style>
</head>
<body>
<h1>PBR Batch Report — {} materials</h1>
{}
<footer>Generated by PBR Studio — {}</footer>
</body>
</html>"#,
        reports.len(),
        items,
        chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")
    )
}

fn severity_class(s: Severity) -> &'static str {
    match s {
        Severity::Critical => "critical",
        Severity::Major => "major",
        Severity::Minor => "minor",
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(all(test, feature = "pdf"))]
mod tests {
    use super::*;
    use crate::json_report::{MaterialReport, MapSummary, MaterialSummary, OptimizationSuggestion, ReportIssue};
    use crate::estimation::VramEstimate;

    fn sample_report() -> MaterialReport {
        MaterialReport {
            name: Some("Sample PBR Material".into()),
            score: 85,
            summary: MaterialSummary {
                texture_count: 5,
                dimensions: Some(crate::json_report::TextureDimensions {
                    width: 1024,
                    height: 1024,
                }),
                maps: MapSummary {
                    albedo: true,
                    normal: true,
                    roughness: true,
                    metallic: true,
                    ao: true,
                    height: false,
                },
                dimensions_consistent: true,
            },
            issues: vec![
                ReportIssue {
                    rule_id: "texture_resolution".into(),
                    severity: crate::json_report::Severity::Minor,
                    message: "Consider 2K for mobile targets".into(),
                },
            ],
            optimization_suggestions: vec![
                OptimizationSuggestion::new("resolution", "Downscale to 2K for faster loading"),
            ],
            passed: true,
            error_count: 0,
            warning_count: 0,
            vram_estimate: Some(VramEstimate {
                bytes: 20_971_520,
                formatted: "20.0 MB".into(),
                include_mipmaps: true,
                packed_orm: false,
                textures: vec![],
            }),
            ai_insights: None,
        }
    }

    /// Tests PDF export on Linux, Windows, and macOS. Uses bundled font when system
    /// fonts are unavailable; fully offline.
    #[test]
    fn export_pdf_single_generates_sample_report() {
        let report = sample_report();
        let out = std::env::temp_dir().join("pbr_studio_sample_report.pdf");
        let result = export_pdf_single(&report, &out);
        assert!(result.is_ok(), "PDF export failed: {:?}", result.err());
        assert!(out.exists(), "PDF file was not created");
        let meta = std::fs::metadata(&out).unwrap();
        assert!(meta.len() > 100, "PDF file appears empty or too small");
        // Verify PDF header (works on all platforms)
        let content = std::fs::read(&out).unwrap();
        assert!(content.starts_with(b"%PDF"), "PDF file has invalid header");
    }

    /// Tests batch PDF export on all platforms. Uses bundled font fallback when needed.
    #[test]
    fn export_pdf_batch_generates_sample_report() {
        let report = sample_report();
        let reports = vec![
            ("materials/wood".into(), report.clone()),
            ("materials/metal".into(), report),
        ];
        let out = std::env::temp_dir().join("pbr_studio_batch_report.pdf");
        let result = export_pdf_batch(&reports, &out);
        assert!(result.is_ok(), "PDF batch export failed: {:?}", result.err());
        assert!(out.exists(), "PDF file was not created");
        let meta = std::fs::metadata(&out).unwrap();
        assert!(meta.len() > 100, "PDF file appears empty or too small");
        let content = std::fs::read(&out).unwrap();
        assert!(content.starts_with(b"%PDF"), "PDF file has invalid header");
    }
}
