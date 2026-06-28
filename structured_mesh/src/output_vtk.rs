//! VTK XML Unstructured Grid (`.vtu`) writer.
//!
//! ## Why VTK?
//!
//! The original Fortran program only produced Tecplot ASCII files.  This module
//! adds a modern [`VTK XML`](https://vtk.org/wp-content/uploads/2015/04/file-formats.pdf)
//! export so the mesh can be opened directly in **ParaView**, **VisIt**, or any
//! VTK-capable tool — without installing Tecplot.
//!
//! ## File structure
//!
//! ```xml
//! <?xml version="1.0" encoding="UTF-8"?>
//! <VTKFile type="UnstructuredGrid" version="0.1" byte_order="LittleEndian">
//!   <UnstructuredGrid>
//!     <Piece NumberOfPoints="N" NumberOfCells="E">
//!       <Points>
//!         <DataArray type="Float64" NumberOfComponents="3" format="ascii">
//!           x y 0.0   ← one triple per node (z=0 for 2-D meshes)
//!         </DataArray>
//!       </Points>
//!       <Cells>
//!         <DataArray Name="connectivity" ...>  ← 4 node indices per quad (0-based) </DataArray>
//!         <DataArray Name="offsets"      ...>  ← cumulative, multiples of 4         </DataArray>
//!         <DataArray Name="types"        ...>  ← all 9 = VTK_QUAD                   </DataArray>
//!       </Cells>
//!       <CellData Scalars="region_tag">
//!         <DataArray Name="region_tag" type="Int32" ...>
//!           ← one integer per element, sourced from PlotData::region_tags
//!         </DataArray>
//!       </CellData>
//!     </Piece>
//!   </UnstructuredGrid>
//! </VTKFile>
//! ```
//!
//! ## Cell type
//!
//! Every element is written as **VTK cell type 9** (`VTK_QUAD`) — a four-node
//! quadrilateral.  Node indices are 0-based (VTK convention), converted from the
//! 1-based indices stored in [`PlotData::elements`](crate::scanline_rasterizer::PlotData).
//!
//! ## Well-formedness guarantee
//!
//! All XML is written through [`quick-xml`](https://docs.rs/quick-xml) which
//! ensures balanced open/close tags.  The `vtu_is_well_formed_xml` unit test
//! parses the output back with `quick-xml` and asserts zero errors.

use std::io::{BufWriter, Write};
use std::path::Path;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;

use crate::error::MeshError;
use crate::scanline_rasterizer::PlotData;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Write a VTK XML Unstructured Grid file (`.vtu`) for the given mesh.
pub fn write_vtu(plot: &PlotData, path: &Path) -> Result<(), MeshError> {
    let file = std::fs::File::create(path).map_err(MeshError::Io)?;
    let buf = BufWriter::new(file);
    let mut w = Writer::new_with_indent(buf, b' ', 2);

    let nnode = plot.nodes.len();
    let nelem = plot.elements.len();

    // <?xml version="1.0"?>
    w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(xml_err)?;
    w.write_event(Event::Text(BytesText::new("\n")))
        .map_err(xml_err)?;

    // <VTKFile type="UnstructuredGrid" version="0.1" byte_order="LittleEndian">
    let mut vtkfile = BytesStart::new("VTKFile");
    vtkfile.push_attribute(("type", "UnstructuredGrid"));
    vtkfile.push_attribute(("version", "0.1"));
    vtkfile.push_attribute(("byte_order", "LittleEndian"));
    w.write_event(Event::Start(vtkfile)).map_err(xml_err)?;

    // <UnstructuredGrid>
    w.write_event(Event::Start(BytesStart::new("UnstructuredGrid")))
        .map_err(xml_err)?;

    // <Piece NumberOfPoints="..." NumberOfCells="...">
    let mut piece = BytesStart::new("Piece");
    piece.push_attribute(("NumberOfPoints", nnode.to_string().as_str()));
    piece.push_attribute(("NumberOfCells", nelem.to_string().as_str()));
    w.write_event(Event::Start(piece)).map_err(xml_err)?;

    // ── <Points> ──────────────────────────────────────────────────────────
    w.write_event(Event::Start(BytesStart::new("Points")))
        .map_err(xml_err)?;

    let mut pts_array = BytesStart::new("DataArray");
    pts_array.push_attribute(("type", "Float64"));
    pts_array.push_attribute(("NumberOfComponents", "3"));
    pts_array.push_attribute(("format", "ascii"));
    w.write_event(Event::Start(pts_array)).map_err(xml_err)?;

    // Build coordinate string: "x y 0.0\n" per node.
    let mut pts_text = String::new();
    pts_text.push('\n');
    for &(x, y) in &plot.nodes {
        pts_text.push_str(&format!("          {} {} 0.0\n", x, y));
    }
    pts_text.push_str("        ");
    w.write_event(Event::Text(BytesText::new(&pts_text)))
        .map_err(xml_err)?;

    w.write_event(Event::End(BytesEnd::new("DataArray")))
        .map_err(xml_err)?;
    w.write_event(Event::End(BytesEnd::new("Points")))
        .map_err(xml_err)?;

    // ── <Cells> ───────────────────────────────────────────────────────────
    w.write_event(Event::Start(BytesStart::new("Cells")))
        .map_err(xml_err)?;

    // connectivity — 4 0-based node indices per element (elements store 1-based)
    let mut conn_array = BytesStart::new("DataArray");
    conn_array.push_attribute(("type", "Int64"));
    conn_array.push_attribute(("Name", "connectivity"));
    conn_array.push_attribute(("format", "ascii"));
    w.write_event(Event::Start(conn_array)).map_err(xml_err)?;

    let mut conn_text = String::new();
    conn_text.push('\n');
    for elem in &plot.elements {
        // elements are 1-based; VTK expects 0-based.
        conn_text.push_str(&format!(
            "          {} {} {} {}\n",
            elem[0] - 1,
            elem[1] - 1,
            elem[2] - 1,
            elem[3] - 1,
        ));
    }
    conn_text.push_str("        ");
    w.write_event(Event::Text(BytesText::new(&conn_text)))
        .map_err(xml_err)?;
    w.write_event(Event::End(BytesEnd::new("DataArray")))
        .map_err(xml_err)?;

    // offsets — cumulative, multiples of 4
    let mut off_array = BytesStart::new("DataArray");
    off_array.push_attribute(("type", "Int64"));
    off_array.push_attribute(("Name", "offsets"));
    off_array.push_attribute(("format", "ascii"));
    w.write_event(Event::Start(off_array)).map_err(xml_err)?;

    let mut off_text = String::new();
    off_text.push('\n');
    for i in 1..=nelem {
        off_text.push_str(&format!("          {}\n", i * 4));
    }
    off_text.push_str("        ");
    w.write_event(Event::Text(BytesText::new(&off_text)))
        .map_err(xml_err)?;
    w.write_event(Event::End(BytesEnd::new("DataArray")))
        .map_err(xml_err)?;

    // types — all 9 (VTK_QUAD)
    let mut typ_array = BytesStart::new("DataArray");
    typ_array.push_attribute(("type", "UInt8"));
    typ_array.push_attribute(("Name", "types"));
    typ_array.push_attribute(("format", "ascii"));
    w.write_event(Event::Start(typ_array)).map_err(xml_err)?;

    let mut typ_text = String::new();
    typ_text.push('\n');
    for _ in 0..nelem {
        typ_text.push_str("          9\n");
    }
    typ_text.push_str("        ");
    w.write_event(Event::Text(BytesText::new(&typ_text)))
        .map_err(xml_err)?;
    w.write_event(Event::End(BytesEnd::new("DataArray")))
        .map_err(xml_err)?;

    w.write_event(Event::End(BytesEnd::new("Cells")))
        .map_err(xml_err)?;

    // ── <CellData Scalars="region_tag"> ───────────────────────────────────
    let mut cell_data = BytesStart::new("CellData");
    cell_data.push_attribute(("Scalars", "region_tag"));
    w.write_event(Event::Start(cell_data)).map_err(xml_err)?;

    let mut tag_array = BytesStart::new("DataArray");
    tag_array.push_attribute(("type", "Int32"));
    tag_array.push_attribute(("Name", "region_tag"));
    tag_array.push_attribute(("format", "ascii"));
    w.write_event(Event::Start(tag_array)).map_err(xml_err)?;

    let mut tag_text = String::new();
    tag_text.push('\n');
    for &tag in &plot.region_tags {
        tag_text.push_str(&format!("          {}\n", tag));
    }
    tag_text.push_str("        ");
    w.write_event(Event::Text(BytesText::new(&tag_text)))
        .map_err(xml_err)?;
    w.write_event(Event::End(BytesEnd::new("DataArray")))
        .map_err(xml_err)?;

    w.write_event(Event::End(BytesEnd::new("CellData")))
        .map_err(xml_err)?;

    // Close open elements.
    w.write_event(Event::End(BytesEnd::new("Piece")))
        .map_err(xml_err)?;
    w.write_event(Event::End(BytesEnd::new("UnstructuredGrid")))
        .map_err(xml_err)?;
    w.write_event(Event::End(BytesEnd::new("VTKFile")))
        .map_err(xml_err)?;

    // Flush the BufWriter.
    w.into_inner().flush().map_err(MeshError::Io)
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn xml_err(e: quick_xml::Error) -> MeshError {
    MeshError::Io(std::io::Error::other(e.to_string()))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input_parser::{Edge, MeshInput};
    use crate::scanline_rasterizer::{build_plot_data, compute_grid, rasterize};
    use tempfile::NamedTempFile;

    fn small_plot() -> PlotData {
        let input = MeshInput {
            dx: 1.0,
            dy: 1.0,
            nodes: vec![(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)],
            edges: vec![
                Edge { k1: 0, k2: 1, kp: 1 },
                Edge { k1: 1, k2: 2, kp: 1 },
                Edge { k1: 2, k2: 3, kp: 1 },
                Edge { k1: 3, k2: 0, kp: 1 },
            ],
        };
        let geo = compute_grid(&input).unwrap();
        let state = rasterize(&input, &geo).unwrap();
        build_plot_data(&state)
    }

    #[test]
    fn vtu_is_well_formed_xml() {
        let plot = small_plot();
        let tmp = NamedTempFile::new().unwrap();
        write_vtu(&plot, tmp.path()).expect("write_vtu must succeed");

        let content = std::fs::read_to_string(tmp.path()).unwrap();

        // quick-xml round-trip: parse without errors.
        let mut reader = quick_xml::Reader::from_str(&content);
        reader.check_end_names(true);
        let mut depth = 0i32;
        loop {
            match reader.read_event() {
                Ok(Event::Start(_)) => depth += 1,
                Ok(Event::End(_)) => depth -= 1,
                Ok(Event::Eof) => break,
                Err(e) => panic!("XML parse error: {e}"),
                _ => {}
            }
        }
        assert_eq!(depth, 0, "XML must be balanced (depth=0 at EOF)");
    }

    #[test]
    fn vtu_cell_types_all_nine() {
        let plot = small_plot();
        let tmp = NamedTempFile::new().unwrap();
        write_vtu(&plot, tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path()).unwrap();

        // All type values in the types DataArray must be "9".
        // Find the types block between Name="types" and its closing tag.
        let start = content.find("Name=\"types\"").expect("types DataArray missing");
        let block = &content[start..];
        let data_start = block.find('>').unwrap() + 1;
        let data_end = block.find("</DataArray>").unwrap();
        let data = &block[data_start..data_end];

        for tok in data.split_whitespace() {
            assert_eq!(tok, "9", "all cell types must be 9 (VTK_QUAD), found: {tok}");
        }
    }

    #[test]
    fn vtu_region_tag_count_matches_elements() {
        let plot = small_plot();
        let nelem = plot.elements.len();
        let tmp = NamedTempFile::new().unwrap();
        write_vtu(&plot, tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path()).unwrap();

        // Find region_tag DataArray data.
        let start = content.find("Name=\"region_tag\"").expect("region_tag missing");
        let block = &content[start..];
        let data_start = block.find('>').unwrap() + 1;
        let data_end = block.find("</DataArray>").unwrap();
        let count = block[data_start..data_end].split_whitespace().count();

        assert_eq!(count, nelem, "region_tag count must equal nelem={nelem}");
    }

    #[test]
    fn vtu_connectivity_is_zero_based() {
        let plot = small_plot();
        let tmp = NamedTempFile::new().unwrap();
        write_vtu(&plot, tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path()).unwrap();

        // Connectivity values must be 0-based (no value < 0, min value == 0).
        let start = content.find("Name=\"connectivity\"").expect("connectivity missing");
        let block = &content[start..];
        let data_start = block.find('>').unwrap() + 1;
        let data_end = block.find("</DataArray>").unwrap();
        let values: Vec<i64> = block[data_start..data_end]
            .split_whitespace()
            .map(|s| s.parse().unwrap())
            .collect();

        assert!(values.iter().all(|&v| v >= 0), "connectivity must be non-negative");
        assert!(values.contains(&0), "connectivity must contain index 0");
    }

    #[test]
    fn vtu_offsets_are_multiples_of_four() {
        let plot = small_plot();
        let tmp = NamedTempFile::new().unwrap();
        write_vtu(&plot, tmp.path()).unwrap();
        let content = std::fs::read_to_string(tmp.path()).unwrap();

        let start = content.find("Name=\"offsets\"").expect("offsets missing");
        let block = &content[start..];
        let data_start = block.find('>').unwrap() + 1;
        let data_end = block.find("</DataArray>").unwrap();

        for (i, tok) in block[data_start..data_end].split_whitespace().enumerate() {
            let v: usize = tok.parse().unwrap();
            assert_eq!(v, (i + 1) * 4, "offset[{i}] must be {}", (i + 1) * 4);
        }
    }
}
