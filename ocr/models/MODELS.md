# Models

PP-OCRv5 mobile text detection and English recognition, from PaddleOCR (Apache-2.0), in the ONNX
conversion RapidOCR publishes (v3.10.0 on ModelScope, `RapidAI/RapidOCR`), and PaddleOCR's
document-orientation classifier, PP-LCNet_x1_0_doc_ori (Apache-2.0), in the ONNX export PaddlePaddle
publishes (`PaddlePaddle/PP-LCNet_x1_0_doc_ori_onnx` on Hugging Face, commit `7330ab7`, its
`inference.onnx`; decision #153). Built into `ethos-parser-ocr`; their SHA-256 is part of its
identity.

| File | Source path at v3.10.0 | SHA-256 |
| --- | --- | --- |
| `ch_PP-OCRv5_det_mobile.onnx` | `onnx/PP-OCRv5/det/ch_PP-OCRv5_det_mobile.onnx` | `4d97c44a20d30a81aad087d6a396b08f786c4635742afc391f6621f5c6ae78ae` |
| `en_PP-OCRv5_rec_mobile.onnx` | `onnx/PP-OCRv5/rec/en_PP-OCRv5_rec_mobile.onnx` | `c3461add59bb4323ecba96a492ab75e06dda42467c9e3d0c18db5d1d21924be8` |
| `PP-LCNet_x1_0_doc_ori.onnx` | `inference.onnx` at `7330ab7039123e46af2dc03154b9969aa412c61d` | `af9a0a4f317ff0709ce752067807f819cb15d883f8ecad89f28df1c6ee2d9c92` |
| `ppocrv5_en_dict.txt` | `paddle/PP-OCRv5/rec/en_PP-OCRv5_rec_mobile/ppocrv5_en_dict.txt` | `e025a66d31f327ba0c232e03f407ae8d105e1e709e7ccb3f408aa778c24e70d6` |
