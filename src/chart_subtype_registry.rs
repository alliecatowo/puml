use crate::model::ChartSubtype;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChartSubtypeSpec {
    pub subtype: ChartSubtype,
    pub id: &'static str,
    pub display_name: &'static str,
    pub default_title: &'static str,
    pub aliases: &'static [&'static str],
    pub docs_example: Option<ChartDocsExample>,
    pub axes: ChartAxisPolicy,
    pub values: ChartValuePolicy,
    pub rendering: ChartRenderingMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChartDocsExample {
    pub folder: &'static str,
    pub id: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChartAxisPolicy {
    pub cartesian_axes: bool,
    pub default_h_axis_label: Option<&'static str>,
    pub default_v_axis_label: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChartValuePolicy {
    pub negative_values: ChartNegativeValuePolicy,
    pub supports_explicit_series: bool,
    pub supports_multi_series: bool,
    pub supports_stacked: bool,
    pub supports_horizontal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartNegativeValuePolicy {
    PreservedOnAxis,
    ClampedToZeroSlices,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChartRenderingMetadata {
    pub renderer: ChartRenderer,
    pub svg_data_chart_type: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartRenderer {
    Bar,
    Line,
    Pie,
    Area,
    Scatter,
}

const CARTESIAN_AXIS_POLICY: ChartAxisPolicy = ChartAxisPolicy {
    cartesian_axes: true,
    default_h_axis_label: Some("Category"),
    default_v_axis_label: Some("Value"),
};

const PIE_AXIS_POLICY: ChartAxisPolicy = ChartAxisPolicy {
    cartesian_axes: false,
    default_h_axis_label: None,
    default_v_axis_label: None,
};

const BAR_VALUES: ChartValuePolicy = ChartValuePolicy {
    negative_values: ChartNegativeValuePolicy::PreservedOnAxis,
    supports_explicit_series: true,
    supports_multi_series: true,
    supports_stacked: true,
    supports_horizontal: true,
};

const LINE_VALUES: ChartValuePolicy = ChartValuePolicy {
    negative_values: ChartNegativeValuePolicy::PreservedOnAxis,
    supports_explicit_series: true,
    supports_multi_series: true,
    supports_stacked: false,
    supports_horizontal: false,
};

const PIE_VALUES: ChartValuePolicy = ChartValuePolicy {
    negative_values: ChartNegativeValuePolicy::ClampedToZeroSlices,
    supports_explicit_series: true,
    supports_multi_series: false,
    supports_stacked: false,
    supports_horizontal: false,
};

const POINT_VALUES: ChartValuePolicy = ChartValuePolicy {
    negative_values: ChartNegativeValuePolicy::PreservedOnAxis,
    supports_explicit_series: false,
    supports_multi_series: false,
    supports_stacked: false,
    supports_horizontal: false,
};

pub static CHART_SUBTYPE_SPECS: &[ChartSubtypeSpec] = &[
    ChartSubtypeSpec {
        subtype: ChartSubtype::Bar,
        id: "bar",
        display_name: "Bar Chart",
        default_title: "Bar Chart",
        aliases: &["bars", "bar chart", "barchart"],
        docs_example: Some(ChartDocsExample {
            folder: "chart",
            id: "01_bar",
        }),
        axes: CARTESIAN_AXIS_POLICY,
        values: BAR_VALUES,
        rendering: ChartRenderingMetadata {
            renderer: ChartRenderer::Bar,
            svg_data_chart_type: "bar",
        },
    },
    ChartSubtypeSpec {
        subtype: ChartSubtype::Line,
        id: "line",
        display_name: "Line Chart",
        default_title: "Line Chart",
        aliases: &["lines", "line chart", "linechart"],
        docs_example: Some(ChartDocsExample {
            folder: "chart",
            id: "02_line",
        }),
        axes: CARTESIAN_AXIS_POLICY,
        values: LINE_VALUES,
        rendering: ChartRenderingMetadata {
            renderer: ChartRenderer::Line,
            svg_data_chart_type: "line",
        },
    },
    ChartSubtypeSpec {
        subtype: ChartSubtype::Pie,
        id: "pie",
        display_name: "Pie Chart",
        default_title: "Pie Chart",
        aliases: &["pie chart", "piechart"],
        docs_example: Some(ChartDocsExample {
            folder: "chart",
            id: "03_pie",
        }),
        axes: PIE_AXIS_POLICY,
        values: PIE_VALUES,
        rendering: ChartRenderingMetadata {
            renderer: ChartRenderer::Pie,
            svg_data_chart_type: "pie",
        },
    },
    ChartSubtypeSpec {
        subtype: ChartSubtype::Area,
        id: "area",
        display_name: "Area Chart",
        default_title: "Area Chart",
        aliases: &["area chart", "areachart"],
        docs_example: None,
        axes: CARTESIAN_AXIS_POLICY,
        values: POINT_VALUES,
        rendering: ChartRenderingMetadata {
            renderer: ChartRenderer::Area,
            svg_data_chart_type: "area",
        },
    },
    ChartSubtypeSpec {
        subtype: ChartSubtype::Scatter,
        id: "scatter",
        display_name: "Scatter Chart",
        default_title: "Scatter Chart",
        aliases: &["scatter chart", "scatterchart"],
        docs_example: None,
        axes: CARTESIAN_AXIS_POLICY,
        values: POINT_VALUES,
        rendering: ChartRenderingMetadata {
            renderer: ChartRenderer::Scatter,
            svg_data_chart_type: "scatter",
        },
    },
];

pub fn chart_subtype_specs() -> &'static [ChartSubtypeSpec] {
    CHART_SUBTYPE_SPECS
}

pub fn chart_subtype_spec(subtype: ChartSubtype) -> &'static ChartSubtypeSpec {
    match subtype {
        ChartSubtype::Bar => &CHART_SUBTYPE_SPECS[0],
        ChartSubtype::Line => &CHART_SUBTYPE_SPECS[1],
        ChartSubtype::Pie => &CHART_SUBTYPE_SPECS[2],
        ChartSubtype::Area => &CHART_SUBTYPE_SPECS[3],
        ChartSubtype::Scatter => &CHART_SUBTYPE_SPECS[4],
    }
}

pub fn lookup_chart_subtype_spec(input: &str) -> Option<&'static ChartSubtypeSpec> {
    let key = normalize_chart_subtype_key(input);
    CHART_SUBTYPE_SPECS
        .iter()
        .find(|spec| spec.id == key.as_str() || spec.aliases.contains(&key.as_str()))
}

fn normalize_chart_subtype_key(input: &str) -> String {
    let mut normalized = String::new();
    for token in input.split_whitespace() {
        if !normalized.is_empty() {
            normalized.push(' ');
        }
        normalized.push_str(&token.to_ascii_lowercase());
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn registry_lists_known_chart_subtypes() {
        let known = [
            ChartSubtype::Bar,
            ChartSubtype::Line,
            ChartSubtype::Pie,
            ChartSubtype::Area,
            ChartSubtype::Scatter,
        ];

        assert_eq!(chart_subtype_specs().len(), known.len());
        for subtype in known {
            let spec = chart_subtype_spec(subtype);
            assert_eq!(spec.subtype, subtype);
            assert!(
                chart_subtype_specs()
                    .iter()
                    .any(|candidate| candidate.subtype == subtype),
                "{subtype:?} missing from chart subtype registry"
            );
        }
    }

    #[test]
    fn registry_ids_and_aliases_are_unique() {
        let mut keys = BTreeSet::new();
        for spec in chart_subtype_specs() {
            assert!(
                keys.insert(spec.id),
                "duplicate chart subtype id `{}`",
                spec.id
            );
            for alias in spec.aliases {
                assert!(
                    keys.insert(alias),
                    "duplicate chart subtype alias `{alias}`"
                );
            }
        }
    }

    #[test]
    fn lookup_accepts_ids_aliases_case_and_extra_space() {
        assert_eq!(
            lookup_chart_subtype_spec("bar").map(|spec| spec.subtype),
            Some(ChartSubtype::Bar)
        );
        assert_eq!(
            lookup_chart_subtype_spec("  BAR   CHART  ").map(|spec| spec.subtype),
            Some(ChartSubtype::Bar)
        );
        assert_eq!(
            lookup_chart_subtype_spec("piechart").map(|spec| spec.subtype),
            Some(ChartSubtype::Pie)
        );
        assert_eq!(lookup_chart_subtype_spec("histogram"), None);
    }

    #[test]
    fn registry_documents_current_axis_and_value_policies() {
        let bar = chart_subtype_spec(ChartSubtype::Bar);
        assert!(bar.axes.cartesian_axes);
        assert_eq!(bar.default_title, "Bar Chart");
        assert_eq!(bar.axes.default_h_axis_label, Some("Category"));
        assert!(bar.values.supports_multi_series);
        assert!(bar.values.supports_horizontal);

        let pie = chart_subtype_spec(ChartSubtype::Pie);
        assert!(!pie.axes.cartesian_axes);
        assert_eq!(
            pie.values.negative_values,
            ChartNegativeValuePolicy::ClampedToZeroSlices
        );
        assert!(!pie.values.supports_multi_series);
    }
}
