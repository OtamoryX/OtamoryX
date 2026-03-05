pub mod comicinfo_parser;
pub mod date_added;
pub mod filename_parser;
pub mod tag_copier;

use crate::plugins::{
    BuiltinPlugin, BuiltinPluginKind, BUILTIN_COMICINFO_PARSER_ID, BUILTIN_DATE_ADDED_ID,
    BUILTIN_FILENAME_PARSER_ID, BUILTIN_METADATA_EXECUTION_ORDER, BUILTIN_TAG_COPIER_ID,
};

pub fn metadata_pipeline_plugins() -> Vec<Box<dyn BuiltinPlugin>> {
    let mut plugins: Vec<Box<dyn BuiltinPlugin>> = vec![
        Box::new(filename_parser::FilenameParser::default()),
        Box::new(comicinfo_parser::ComicInfoParser::default()),
        Box::new(date_added::DateAdded::default()),
    ];

    plugins.sort_by_key(|plugin| plugin.order().unwrap_or(u16::MAX));
    plugins
}

pub fn utility_plugins() -> Vec<Box<dyn BuiltinPlugin>> {
    vec![Box::new(tag_copier::TagCopier::default())]
}

pub fn is_metadata_pipeline_plugin(plugin_id: &str) -> bool {
    matches!(
        plugin_id,
        BUILTIN_FILENAME_PARSER_ID | BUILTIN_COMICINFO_PARSER_ID | BUILTIN_DATE_ADDED_ID
    )
}

pub fn is_utility_plugin(plugin_id: &str) -> bool {
    plugin_id == BUILTIN_TAG_COPIER_ID
}

pub fn expected_metadata_pipeline_order() -> [(&'static str, u16); 3] {
    BUILTIN_METADATA_EXECUTION_ORDER
}

pub fn plugin_kind(plugin_id: &str) -> Option<BuiltinPluginKind> {
    if is_metadata_pipeline_plugin(plugin_id) {
        return Some(BuiltinPluginKind::MetadataPipeline);
    }
    if is_utility_plugin(plugin_id) {
        return Some(BuiltinPluginKind::Utility);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_plugins_follow_expected_execution_order() {
        let plugins = metadata_pipeline_plugins();
        let actual = plugins
            .iter()
            .map(|plugin| (plugin.id(), plugin.order().unwrap_or(u16::MAX)))
            .collect::<Vec<_>>();
        let expected = expected_metadata_pipeline_order().to_vec();

        assert_eq!(actual, expected);
    }

    #[test]
    fn plugin_kind_detection_matches_builtin_groups() {
        assert_eq!(
            plugin_kind(BUILTIN_FILENAME_PARSER_ID),
            Some(BuiltinPluginKind::MetadataPipeline)
        );
        assert_eq!(
            plugin_kind(BUILTIN_TAG_COPIER_ID),
            Some(BuiltinPluginKind::Utility)
        );
        assert_eq!(plugin_kind("unknown-plugin"), None);
    }
}
