// Copyright 2026 a7mddra
// SPDX-License-Identifier: Apache-2.0

use ops_chat_storage::{ChatData, ChatMetadata, ChatStorage};
use ops_squigit_ocr::ocr::{boxes_to_storage_regions, persist_boxes_to_chat_storage, OcrBox};

#[test]
fn cli_style_ocr_write_is_renderable_through_shared_chat_storage_frame() {
    let temp = tempfile::tempdir().expect("tempdir");
    let storage_root = temp.path().join("chats");
    let storage = ChatStorage::with_base_dir(storage_root).expect("chat storage");

    let metadata = ChatMetadata::new(
        "Parity Chat".to_string(),
        "deadbeef".to_string(),
        Some("pp-ocr-v5-en".to_string()),
    );
    storage
        .save_chat(&ChatData::new(metadata.clone()))
        .expect("save chat");

    let boxes = vec![OcrBox {
        text: "Hello from cli renderer".to_string(),
        box_coords: vec![
            vec![10.2, 20.8],
            vec![30.1, 20.2],
            vec![30.9, 40.7],
            vec![10.3, 40.2],
        ],
        confidence: 0.99,
    }];
    persist_boxes_to_chat_storage(&storage, &metadata.id, "pp-ocr-v5-en", &boxes)
        .expect("save ocr frame");
    let regions = boxes_to_storage_regions(&boxes);

    let frame = storage.get_ocr_frame(&metadata.id).expect("load frame");
    let restored = frame
        .get("pp-ocr-v5-en")
        .and_then(|value| value.clone())
        .expect("model frame should be populated");

    assert_eq!(restored.len(), regions.len());
    assert_eq!(restored[0].text, regions[0].text);
    assert_eq!(restored[0].bbox, regions[0].bbox);
}

#[test]
fn invalid_model_id_is_still_rejected_by_shared_storage_contract() {
    let temp = tempfile::tempdir().expect("tempdir");
    let storage = ChatStorage::with_base_dir(temp.path().join("chats")).expect("chat storage");

    let result = storage.save_ocr_data("chat-1", "unsupported-model", &[]);
    assert!(result.is_err(), "invalid model IDs must keep failing");
}
