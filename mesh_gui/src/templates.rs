// Hardcoded example file contents for template generation (Req 3 AC6).

/// Example `geometry.txt` for the Structured Mesh engine.
///
/// Defines a unit square with `dx = dy = 0.1`, four nodes in CCW order,
/// and four directed edges.
pub const STRUCTURED_TEMPLATE: &str = "\
0.1 0.1
4
0.0 0.0
1.0 0.0
1.0 1.0
0.0 1.0
4
1 2 1
2 3 1
3 4 1
4 1 1
";

/// Example `.d` input file for the EasyMesh engine.
///
/// Verbatim content of `EasyMesh/Examples/example1.d`: a quadrilateral outer
/// boundary with a rectangular hole.
pub const EASYMESH_TEMPLATE: &str = "\
#-----------#
# Example 1 #
#-----------#

#=========
| POINTS |
=========#
9 # number of points #

# Nodes which define the boundary #
0:   0   0   0.25   1
1:   5   0   0.25   1
2:   5   2   0.25   2
3:   4   3   0.25   3
4:   0   3   0.25   3

# Nodes which define the hole #
5:   1   1   0.1    4
6:   1   2   0.1    4
7:   2   2   0.1    4
8:   2   1   0.1    4

#===========
| SEGMENTS |
===========#
9 # Number of segments #

# Boundary segments #
0:   0   1   1
1:   1   2   2
2:   2   3   2
3:   3   4   3
4:   4   0   3

# Hole segments #
5:   5   6   4
6:   6   7   4
7:   7   8   4
8:   8   5   4
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_template_is_non_empty() {
        assert!(!STRUCTURED_TEMPLATE.is_empty());
    }

    #[test]
    fn structured_template_contains_dx_dy() {
        assert!(
            STRUCTURED_TEMPLATE.contains("0.1"),
            "STRUCTURED_TEMPLATE should contain '0.1' (the dx/dy value)"
        );
    }

    #[test]
    fn structured_template_has_expected_structure() {
        // Check dx/dy line
        assert!(STRUCTURED_TEMPLATE.contains("0.1 0.1"));
        // Check node count
        assert!(STRUCTURED_TEMPLATE.contains("4\n"));
        // Check a CCW edge line
        assert!(STRUCTURED_TEMPLATE.contains("1 2 1"));
    }

    #[test]
    fn easymesh_template_is_non_empty() {
        assert!(!EASYMESH_TEMPLATE.is_empty());
    }

    #[test]
    fn easymesh_template_contains_points_section() {
        assert!(
            EASYMESH_TEMPLATE.contains("POINTS"),
            "EASYMESH_TEMPLATE should contain 'POINTS'"
        );
    }

    #[test]
    fn easymesh_template_contains_point_entries() {
        assert!(
            EASYMESH_TEMPLATE.contains("0:"),
            "EASYMESH_TEMPLATE should contain '0:' (first point/segment entry)"
        );
    }

    #[test]
    fn easymesh_template_contains_segments_section() {
        assert!(
            EASYMESH_TEMPLATE.contains("SEGMENTS"),
            "EASYMESH_TEMPLATE should contain 'SEGMENTS'"
        );
    }

    #[test]
    fn easymesh_template_has_nine_points() {
        assert!(EASYMESH_TEMPLATE.contains("9 # number of points #"));
    }

    #[test]
    fn easymesh_template_has_nine_segments() {
        assert!(EASYMESH_TEMPLATE.contains("9 # Number of segments #"));
    }
}
