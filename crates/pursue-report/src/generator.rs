//! Report generators: deterministic JSON and printable self-contained HTML.
//!
//! Enforces zero external dependencies in generated HTML (no external JS, CSS, or fonts).
//! All text inputs are rigorously HTML-escaped.

use crate::model::InvestigationReport;
use pursue_core::{Error, Result};
use std::fmt::Write as _;

/// Supported forensic report export formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportFormat {
    /// Canonical JSON document.
    Json,
    /// Standalone printable HTML report.
    Html,
}

impl ReportFormat {
    /// Parses format string ("json" or "html").
    pub fn parse(s: &str) -> Result<Self> {
        match s.trim().to_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "html" => Ok(Self::Html),
            other => Err(Error::InvalidInput(format!(
                "unsupported report format '{other}'; must be 'json' or 'html'"
            ))),
        }
    }
}

/// Helper to escape raw text for HTML embedding to prevent injection.
pub fn escape_html(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// Renders an [`InvestigationReport`] as a pretty-printed, deterministic JSON string.
pub fn render_json(report: &InvestigationReport) -> Result<String> {
    serde_json::to_string_pretty(report)
        .map_err(|e| Error::InvalidInput(format!("failed to serialize report to JSON: {e}")))
}

/// Renders an [`InvestigationReport`] as a self-contained, printable forensic HTML document.
pub fn render_html(report: &InvestigationReport) -> Result<String> {
    let mut html = String::with_capacity(16 * 1024);

    let report_hash = report.report_hash.as_deref().unwrap_or("UNFINALIZED");

    let audit_badge_color = if report.audit_summary.chain_intact {
        "#107c41"
    } else {
        "#d83b01"
    };
    let audit_status_text = if report.audit_summary.chain_intact {
        "VERIFIED INTACT"
    } else {
        "VERIFICATION FAILED"
    };

    let status_color = match report.metadata.case_status.as_str() {
        "open" => "#0078d4",
        "closed" => "#5d5a58",
        _ => "#2d3748",
    };

    writeln!(
        html,
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>PURSUE OS — Forensic Report [{case_id}]</title>
<style>
  :root {{
    --bg-main: #ffffff;
    --text-main: #1a202c;
    --text-muted: #718096;
    --border: #e2e8f0;
    --card-bg: #f8fafc;
    --accent: #0f172a;
  }}
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    color: var(--text-main);
    background-color: var(--bg-main);
    line-height: 1.5;
    padding: 2rem;
    max-width: 1000px;
    margin: 0 auto;
  }}
  header {{
    border-bottom: 2px solid var(--accent);
    padding-bottom: 1.5rem;
    margin-bottom: 2rem;
  }}
  .brand {{
    font-size: 0.85rem;
    font-weight: 700;
    letter-spacing: 0.1em;
    color: var(--text-muted);
    text-transform: uppercase;
  }}
  h1 {{
    font-size: 1.8rem;
    font-weight: 800;
    color: var(--accent);
    margin-top: 0.25rem;
  }}
  .badge {{
    display: inline-block;
    padding: 0.2rem 0.6rem;
    font-size: 0.75rem;
    font-weight: 700;
    border-radius: 4px;
    color: #fff;
    text-transform: uppercase;
    vertical-align: middle;
  }}
  .section {{
    margin-bottom: 2.5rem;
  }}
  h2 {{
    font-size: 1.25rem;
    font-weight: 700;
    color: var(--accent);
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.5rem;
    margin-bottom: 1rem;
  }}
  .grid {{
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 1rem;
  }}
  .card {{
    background-color: var(--card-bg);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 1rem;
  }}
  .card-label {{
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
  }}
  .card-value {{
    font-size: 1rem;
    font-weight: 600;
    margin-top: 0.25rem;
    word-break: break-all;
  }}
  .hash-box {{
    background-color: #0f172a;
    color: #38bdf8;
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 0.85rem;
    padding: 0.75rem 1rem;
    border-radius: 6px;
    word-break: break-all;
    margin-top: 0.5rem;
  }}
  table {{
    width: 100%;
    border-collapse: collapse;
    margin-top: 0.75rem;
    font-size: 0.9rem;
  }}
  th, td {{
    padding: 0.75rem;
    text-align: left;
    border-bottom: 1px solid var(--border);
    vertical-align: top;
  }}
  th {{
    background-color: var(--card-bg);
    font-weight: 600;
    color: var(--text-muted);
    font-size: 0.8rem;
    text-transform: uppercase;
  }}
  .mono {{
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 0.85rem;
  }}
  .preview-box {{
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
    font-size: 0.8rem;
    background: #f1f5f9;
    padding: 0.5rem;
    border-radius: 4px;
    margin-top: 0.25rem;
    white-space: pre-wrap;
    max-height: 120px;
    overflow-y: auto;
  }}
  footer {{
    margin-top: 3rem;
    border-top: 1px solid var(--border);
    padding-top: 1rem;
    font-size: 0.8rem;
    color: var(--text-muted);
    display: flex;
    justify-content: space-between;
  }}
  @media print {{
    body {{ padding: 0; }}
    .badge {{ -webkit-print-color-adjust: exact; print-color-adjust: exact; }}
    .hash-box {{ -webkit-print-color-adjust: exact; print-color-adjust: exact; background-color: #e2e8f0 !important; color: #000 !important; }}
    table {{ page-break-inside: auto; }}
    tr {{ page-break-inside: avoid; page-break-after: auto; }}
  }}
</style>
</head>
<body>
<header>
  <div class="brand">PURSUE OS &bull; FORENSIC INVESTIGATION REPORT</div>
  <h1>Case: {case_title} <span class="badge" style="background-color: {status_color};">{case_status}</span></h1>
  <div style="font-size: 0.9rem; color: var(--text-muted); margin-top: 0.5rem;">
    Case Identifier: <strong class="mono">{case_id}</strong> &bull; Generated by <strong>{generated_by}</strong> at Unix timestamp <strong>{generated_at}</strong>
  </div>
</header>
"#,
        case_id = escape_html(&report.metadata.case_id),
        case_title = escape_html(&report.metadata.case_title),
        status_color = status_color,
        case_status = escape_html(&report.metadata.case_status),
        generated_by = escape_html(&report.metadata.generated_by),
        generated_at = report.metadata.generated_at_unix,
    )
    .map_err(|e| Error::InvalidInput(e.to_string()))?;

    // Cryptographic Seal Section
    writeln!(
        html,
        r#"<div class="section">
  <h2>Cryptographic Integrity Seal</h2>
  <div class="card">
    <div class="card-label">Report SHA-256 Canonical Digest</div>
    <div class="hash-box">{report_hash}</div>
    <div style="margin-top: 1rem;" class="grid">
      <div>
        <div class="card-label">Audit Log Provenance Status</div>
        <div style="margin-top: 0.25rem;">
          <span class="badge" style="background-color: {audit_badge_color};">{audit_status_text}</span>
          <span style="font-size: 0.85rem; color: var(--text-muted); margin-left: 0.5rem;">({total_audit_events} chained events)</span>
        </div>
      </div>
      <div>
        <div class="card-label">Audit Chain Head Hash</div>
        <div class="mono" style="font-size: 0.8rem; margin-top: 0.25rem;">{head_hash}</div>
      </div>
    </div>
  </div>
</div>"#,
        report_hash = escape_html(report_hash),
        audit_badge_color = audit_badge_color,
        audit_status_text = audit_status_text,
        total_audit_events = report.audit_summary.total_events,
        head_hash = escape_html(report.audit_summary.head_hash.as_deref().unwrap_or("none")),
    )
    .map_err(|e| Error::InvalidInput(e.to_string()))?;

    // Case Details & Notes Section
    let formatted_notes = if report.metadata.case_notes.trim().is_empty() {
        "<em>No investigator notes recorded.</em>".to_string()
    } else {
        escape_html(&report.metadata.case_notes)
    };

    writeln!(
        html,
        r#"<div class="section">
  <h2>Case Details & Notes</h2>
  <div class="grid" style="margin-bottom: 1rem;">
    <div class="card">
      <div class="card-label">Created By</div>
      <div class="card-value">{created_by}</div>
    </div>
    <div class="card">
      <div class="card-label">Created At (Unix)</div>
      <div class="card-value">{created_at}</div>
    </div>
    <div class="card">
      <div class="card-label">Attached Evidence Count</div>
      <div class="card-value">{evidence_count}</div>
    </div>
  </div>
  <div class="card">
    <div class="card-label">Investigator Notes</div>
    <div style="margin-top: 0.5rem; font-size: 0.95rem; white-space: pre-wrap;">{case_notes}</div>
  </div>
</div>"#,
        created_by = escape_html(&report.metadata.created_by),
        created_at = report.metadata.created_at_unix,
        evidence_count = report.metadata.evidence_count,
        case_notes = formatted_notes,
    )
    .map_err(|e| Error::InvalidInput(e.to_string()))?;

    // Evidence Inventory Section
    writeln!(
        html,
        r#"<div class="section">
  <h2>Evidence Inventory ({evidence_count})</h2>
  <table>
    <thead>
      <tr>
        <th>Content Address (SHA-256)</th>
        <th>Size</th>
        <th>Acquired (Unix)</th>
        <th>Source</th>
        <th>Status</th>
      </tr>
    </thead>
    <tbody>"#,
        evidence_count = report.evidence_items.len(),
    )
    .map_err(|e| Error::InvalidInput(e.to_string()))?;

    if report.evidence_items.is_empty() {
        writeln!(
            html,
            r#"<tr><td colspan="5" style="text-align: center; color: var(--text-muted); padding: 1.5rem;">No evidence items attached to this case.</td></tr>"#
        )
        .map_err(|e| Error::InvalidInput(e.to_string()))?;
    } else {
        for ev in &report.evidence_items {
            let preview_html = if let Some(preview) = &ev.preview {
                format!(r#"<div class="preview-box">{}</div>"#, escape_html(preview))
            } else {
                String::new()
            };

            let status_badge = if ev.verified {
                r#"<span class="badge" style="background-color: #107c41;">VERIFIED</span>"#
            } else {
                r#"<span class="badge" style="background-color: #d83b01;">UNVERIFIED</span>"#
            };

            writeln!(
                html,
                r#"<tr>
  <td>
    <div class="mono" style="font-weight: 600;">{addr}</div>
    {preview_html}
  </td>
  <td class="mono">{size} B</td>
  <td class="mono">{acquired}</td>
  <td>{source}</td>
  <td>{status_badge}</td>
</tr>"#,
                addr = escape_html(&ev.address),
                preview_html = preview_html,
                size = ev.size_bytes,
                acquired = ev.acquired_at_unix,
                source = escape_html(&ev.source),
                status_badge = status_badge,
            )
            .map_err(|e| Error::InvalidInput(e.to_string()))?;
        }
    }

    writeln!(html, r#"    </tbody></table></div>"#)
        .map_err(|e| Error::InvalidInput(e.to_string()))?;

    // Chronological Timeline Section
    writeln!(
        html,
        r#"<div class="section">
  <h2>Investigation Timeline ({timeline_count})</h2>
  <table>
    <thead>
      <tr>
        <th>Seq</th>
        <th>Timestamp (Unix)</th>
        <th>Category</th>
        <th>Actor</th>
        <th>Description</th>
      </tr>
    </thead>
    <tbody>"#,
        timeline_count = report.timeline_items.len(),
    )
    .map_err(|e| Error::InvalidInput(e.to_string()))?;

    for item in &report.timeline_items {
        let type_badge_color = match item.event_type.as_str() {
            "case" => "#0284c7",
            "evidence" => "#0d9488",
            "terminal" => "#475569",
            "browser" => "#7c3aed",
            _ => "#64748b",
        };

        let ref_html = if let Some(ref sid) = item.source_id {
            format!(
                r#" <div class="mono" style="font-size: 0.75rem; color: var(--text-muted);">Ref: {}</div>"#,
                escape_html(sid)
            )
        } else {
            String::new()
        };

        writeln!(
            html,
            r#"<tr>
  <td class="mono">#{seq}</td>
  <td class="mono">{ts}</td>
  <td><span class="badge" style="background-color: {color};">{category}</span></td>
  <td><strong>{actor}</strong></td>
  <td>
    <strong>{title}</strong>
    <div style="font-size: 0.85rem; margin-top: 0.15rem;">{summary}</div>
    {ref_html}
  </td>
</tr>"#,
            seq = item.seq,
            ts = item.timestamp_unix,
            color = type_badge_color,
            category = escape_html(&item.event_type),
            actor = escape_html(&item.actor),
            title = escape_html(&item.title),
            summary = escape_html(&item.summary),
            ref_html = ref_html,
        )
        .map_err(|e| Error::InvalidInput(e.to_string()))?;
    }

    writeln!(
        html,
        r#"    </tbody>
  </table>
</div>
<footer>
  <div>PURSUE OS &bull; Immutable Evidence System</div>
  <div>Report Hash: <span class="mono">{report_hash}</span></div>
</footer>
</body>
</html>"#,
        report_hash = escape_html(report_hash),
    )
    .map_err(|e| Error::InvalidInput(e.to_string()))?;

    Ok(html)
}
