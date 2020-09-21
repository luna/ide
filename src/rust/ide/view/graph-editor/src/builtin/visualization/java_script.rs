//! Example of the visualization JS wrapper API usage
// TODO remove once we have proper visualizations or replace with a nice d3 example.
// These implementations are neither efficient nor pretty, but get the idea across.

use crate::data;
use crate::component::visualization;



///////////////////////////////////////
// JavaScript builtin visualizations //
///////////////////////////////////////

/// Return a `JavaScript` Table view visualization.
pub fn table_view_visualization() -> visualization::java_script::FallibleDefinition {
    let source = include_str!("java_script/tableView.js");

    visualization::java_script::Definition::new(data::builtin_library(),source)
}

/// Return a `JavaScript` Scatterplot visualization.
pub fn scatter_plot() -> visualization::java_script::FallibleDefinition {
    let source = include_str!("java_script/scatterplot.js");

    visualization::java_script::Definition::new(data::builtin_library(),source)
}

/// Return a `JavaScript` Bubble visualization.
pub fn bubble_visualization() -> visualization::java_script::FallibleDefinition {
    let source = include_str!("java_script/bubbleVisualization.js");

    visualization::java_script::Definition::new(data::builtin_library(),source)
}

/// Return an empty minimal `JavaScript` visualization. This should not be used except for testing.
pub fn empty_visualization() -> visualization::java_script::FallibleDefinition {
    let source = r#"
        class EmptyVisualization extends Visualization {}
        return EmptyVisualization;
    "#;

    visualization::java_script::Definition::new(data::builtin_library(),source)
}
