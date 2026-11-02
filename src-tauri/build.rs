fn main() {
    let attributes =
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[
            "get_foundation_status",
            "create_backup",
            "prepare_restore",
            "restore_backup",
            "set_shortcuts",
            "set_auto_save_images",
            "set_launch_at_login",
            "list_items",
            "save_note",
            "export_image",
            "update_note",
            "set_theme",
            "import_image",
            "paste_image",
            "load_item_image",
            "copy_item",
            "open_item",
            "delete_item",
            "set_item_archived",
            "set_item_category",
            "create_category",
            "rename_category",
            "delete_category",
            "hide_panel",
            "drag_panel",
        ]));
    tauri_build::try_build(attributes).expect("could not build Someday permissions");
}
