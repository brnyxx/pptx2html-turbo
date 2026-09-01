<div align="center">

<img src=".github/assets/hero.svg" alt="pptx2html-turbo — 서버 없이 PPTX를 HTML로. WebAssembly로 컴파일되는 순수 Rust ECMA-376 렌더러." width="920">

[![npm](https://img.shields.io/npm/v/@briank-dev/pptx-to-html?style=flat-square&labelColor=0B0F0E&color=F2703C&label=npm)](https://www.npmjs.com/package/@briank-dev/pptx-to-html) [![Rust](https://img.shields.io/badge/rust-2024_edition-8FD9AE?style=flat-square&labelColor=0B0F0E)](Cargo.toml) [![WebAssembly](https://img.shields.io/badge/wasm-browser_ready-F0C24E?style=flat-square&labelColor=0B0F0E)](https://brnyxx.github.io/pptx2html-turbo/) [![Formats](https://img.shields.io/badge/formats-7-9FB3A9?style=flat-square&labelColor=0B0F0E)](#지원-형식) [![License](https://img.shields.io/badge/license-MIT-ECF1EE?style=flat-square&labelColor=0B0F0E)](LICENSE)

[English](README.md) · **한국어**

</div>

`pptx2html-turbo`는 PowerPoint 문서를 독립 실행형 HTML로 변환합니다. 렌더러는
[ECMA-376](https://ecma-international.org/publications-and-standards/standards/ecma-376/)
표준을 보고 Rust로 처음부터 구현했습니다. PPTX 경로에는 PowerPoint도, LibreOffice도, 서버도
개입하지 않습니다. WebAssembly로 컴파일되므로 브라우저 탭 안에서 변환이 끝나며 파일이 기기를
떠나지 않습니다.

렌더러가 정확히 해석하지 못한 요소는 조용히 버리거나 임의로 대체하지 않고, 코드가 부여된
진단(diagnostic)으로 보고합니다.

```console
$ pptx2html deck.pptx -o deck.html
Conversion complete: deck.pptx -> deck.html

$ pptx2html --info deck.pptx
{"slide_count":2,"width_px":960.0,"height_px":720.0,"title":null}
```

**[라이브 데모](https://brnyxx.github.io/pptx2html-turbo/)** — 설치 없이 브라우저에서 `.pptx` 파일을 끌어다 놓아 보세요.
**[릴리스](https://github.com/brnyxx/pptx2html-turbo/releases)** — CLI 아티팩트와 버전별 릴리스 노트.

## 설치

```bash
# npm (WASM — 브라우저)
npm install @briank-dev/pptx-to-html@2.1.0

# CLI (체크아웃한 v2.1.0 소스 트리에서)
cargo install --path crates/pptx2html-cli

# Python (maturin 필요)
cd crates/pptx2html-py && maturin develop

# WASM (소스에서 빌드)
cd crates/pptx2html-wasm && wasm-pack build --target web

# 통합 문서 Python 모듈 (maturin 필요)
cd crates/document2html-py && maturin develop

# 통합 문서 브라우저 WASM 패키지
wasm-pack build crates/document2html-wasm --target web --release
```

기존 `@briank-dev/pptx2html-turbo` 설치도 계속 지원되며, 패키지명 이전 기간 동안 동일한 빌드를
받습니다.

v2.1.0에서 Rust 크레이트와 Python 바인딩은 소스 배포 형태입니다. 이번 릴리스는 crates.io나
PyPI에 게시하지 않습니다. Rust 라이브러리 사용자는 릴리스 태그를 직접 참조할 수 있습니다.

```toml
[dependencies]
pptx2html-core = { git = "https://github.com/brnyxx/pptx2html-turbo", tag = "v2.1.0" }
```

## 지원 형식

PPTX 엔진은 순수 Rust로 작성되어 브라우저를 포함한 어디서든 동작합니다. 나머지 여섯 형식은
설치된 LibreOffice와 Poppler 실행 파일을 호출하는 선택적 네이티브 파이프라인을 거치므로
브라우저 WASM에서는 **사용할 수 없습니다**.

| 형식 | 네이티브 CLI/Python | 브라우저 WASM | 백엔드 |
|---|---|---|---|
| PPTX | 지원 | 지원 | 순수 Rust ECMA-376 렌더러 |
| DOCX | 지원 | 백엔드 없음 | LibreOffice + Poppler |
| DOC | 지원 | 백엔드 없음 | LibreOffice + Poppler |
| XLSX | 지원 | 백엔드 없음 | LibreOffice + Poppler |
| XLS | 지원 | 백엔드 없음 | LibreOffice + Poppler |
| PPT | 지원 | 백엔드 없음 | LibreOffice + Poppler |
| PDF | 지원 | 백엔드 없음 | Poppler |

`document2html-core`는 내용 기반 형식 판별, 공통 결과 타입, 기능 보고, PPTX 어댑터를 담당합니다.
`document2html-native`는 경계가 정해진 Office → PDF → HTML 파이프라인을 추가합니다. 네이티브
경로에는 `soffice`, `pdftohtml`, `pdfinfo`가 필요하며, 격리된 LibreOffice 프로필, 크기가 제한된
임시 작업 공간, 프로세스 타임아웃, 로그/출력 상한, 결정적 자산 이름을 사용하고, 지원되는
런처가 있는 환경에서는 원격 네트워크 접근을 차단합니다.

레거시 XLS 입력은 PDF로 렌더링하기 전에 캐시된 수식 값을 보존하고 워크북 재계산을 비활성화한,
크기가 제한된 XLSX 스냅샷으로 먼저 변환합니다.

```bash
cargo run -p pptx2html-cli --bin document2html -- report.docx -o report.html
cargo run -p pptx2html-cli --bin document2html -- workbook.xls --no-embed
cargo run -p pptx2html-cli --bin document2html -- document.pdf --info
```

기존 `pptx2html` 바이너리와 Rust, Python, npm/WASM의 모든 PPTX API는 그대로 유지됩니다. 전체
계약은 [통합 문서 변환](docs/UNIVERSAL_DOCUMENTS.md) 문서를 참고하세요.

## 사용법

### CLI

```bash
pptx2html input.pptx -o output.html      # 기본 변환
pptx2html input.pptx                     # 기본 출력: input.html
pptx2html input.pptx --slides 1,3,5-8    # 특정 슬라이드만 선택
pptx2html input.pptx --format multi -o output_dir/
pptx2html input.pptx --no-embed          # 이미지를 images/slide-N/ 에 분리 저장
pptx2html input.pptx --include-hidden
pptx2html input.pptx --scale 2.0         # 좌표 변경 없는 슬라이드 전체 확대
pptx2html input.pptx --info              # 프레젠테이션 메타데이터를 JSON으로 출력
pptx2html input.pptx --diagnostics diagnostics.json
pptx2html input.pptx --fail-on-fallback  # 폴백 진단이 있으면 종료 코드 2
```

### 브라우저

```html
<iframe id="output" sandbox="allow-scripts" title="Converted slide output"></iframe>
<script type="module">
import { pptxToHtml } from '@briank-dev/pptx-to-html';

const response = await fetch('/presentation.pptx');
const html = await pptxToHtml(await response.blob());
document.getElementById('output').srcdoc = html;
</script>
```

변환 결과물은 실행 가능한 신뢰할 수 없는 HTML입니다. 위 예시처럼 샌드박스 iframe에서 렌더링하고
샌드박스에 `allow-same-origin`을 넣지 마세요.

<details>
<summary><b>Rust 라이브러리</b></summary>

```rust
use std::{fs, path::Path};
use pptx2html_core::{
    convert_file, convert_file_with_options_metadata, get_info, ConversionOptions,
};

// 기본 변환
let html = convert_file(Path::new("presentation.pptx"))?;

// 바이트에서 변환
let html = pptx2html_core::convert_bytes(&pptx_data)?;

// 옵션 지정
let opts = ConversionOptions {
    embed_images: false,
    include_hidden: true,
    slide_indices: Some(vec![1, 3, 5]),
    scale: 2.0,
    ..Default::default()
};
let result = convert_file_with_options_metadata(Path::new("presentation.pptx"), &opts)?;
let output_dir = Path::new("output");
fs::create_dir_all(output_dir)?;
fs::write(output_dir.join("presentation.html"), &result.html)?;
for asset in &result.external_assets {
    let asset_path = output_dir.join(&asset.relative_path);
    if let Some(parent) = asset_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(asset_path, &asset.data)?;
}

// 메타데이터 조회
let info = get_info(Path::new("presentation.pptx"))?;
println!("Slides: {}, Size: {}x{}", info.slide_count, info.width_px, info.height_px);

// 순서가 보장된 보존 진단과 레거시 unresolved 사이드밴드를 함께 반환
let result = pptx2html_core::convert_file_with_metadata(Path::new("presentation.pptx"))?;
println!("HTML length: {}", result.html.len());
let diagnostic_codes = result.diagnostics.iter().map(|item| item.code.as_str()).collect::<Vec<_>>();
for elem in &result.unresolved_elements {
    println!("Unresolved: {:?} at slide {}", elem.element_type, elem.slide_index);
}
```

Rust API는 파일 경로와 바이트 슬라이스에 같은 작업을 제공합니다.

| 작업 | 파일 경로 | 바이트 |
|---|---|---|
| HTML만 반환 | `convert_file` | `convert_bytes` |
| `ConversionOptions`로 HTML 반환 | `convert_file_with_options` | `convert_bytes_with_options` |
| HTML과 메타데이터 반환 | `convert_file_with_metadata` | `convert_bytes_with_metadata` |
| 옵션과 메타데이터 함께 사용 | `convert_file_with_options_metadata` | `convert_bytes_with_options_metadata` |
| 프레젠테이션 정보 조회 | `get_info` | `get_info_from_bytes` |

`ConversionOptions`에는 `embed_images`, `include_hidden`, 1부터 시작하는 양끝 포함
`slide_range`, 1부터 시작하는 `slide_indices`, 전체 슬라이드 `scale`이 있습니다.
`ConversionResult`에는 `html`, `external_assets`, `font_resolution_entries`,
`provenance_entries`, 순서가 보장된 `diagnostics`, `unresolved_elements`, `slide_count`가
있습니다.

</details>

<details>
<summary><b>Python</b></summary>

```python
import pptx2html

# 기본 변환
html = pptx2html.convert_file("presentation.pptx")

# 바이트에서 변환
html = pptx2html.convert_bytes(pptx_data)

# 옵션 지정
html = pptx2html.convert(
    "presentation.pptx",
    embed_images=False,
    include_hidden=True,
    slides=[1, 3, 5],
    scale=2.0,
)

# 메타데이터 조회
info = pptx2html.get_info("presentation.pptx")
print(f"Slides: {info.slide_count}, Size: {info.width_px}x{info.height_px}")

# 메타데이터 포함 변환 (SmartArt/OLE/Math/사용자 정의 도형 사이드밴드)
result = pptx2html.convert_with_metadata("presentation.pptx")
print(f"HTML: {len(result.html)} chars, Unresolved: {len(result.unresolved_elements)}")
for elem in result.unresolved_elements:
    print(f"  {elem.element_type} at slide {elem.slide_index}: {elem.placeholder_id}")
```

</details>

<details>
<summary><b>저수준 WASM API</b></summary>

```html
<script type="module">
import init, {
  convert,
  convert_with_options,
  convert_with_metadata,
  get_presentation_info,
} from '@briank-dev/pptx-to-html';

await init();

const response = await fetch('presentation.pptx');
const data = new Uint8Array(await response.arrayBuffer());

// 기본 변환
const html = convert(data);
document.getElementById('output').srcdoc = html;

// 옵션 지정 (embedImages, includeHidden, slideIndices, scale)
const html2 = convert_with_options(data, false, true, new Uint32Array([1, 3]), 1.5);

// 타입이 있는 메타데이터
const info = get_presentation_info(data);
console.log(`Slides: ${info.slideCount}, Size: ${info.widthPx}x${info.heightPx}`);

// 메타데이터 사이드밴드 포함 변환 (SmartArt/OLE/Math/사용자 정의 도형)
const result = convert_with_metadata(data);
console.log(`HTML: ${result.html.length}, Unresolved: ${result.unresolvedElements}`);
</script>
```

`crates/pptx2html-wasm/demo/index.html`에 단일 파일 데모 페이지가 포함되어 있습니다. 프로젝트
소개와 실제로 동작하는 드래그 앤 드롭 변환기를 함께 담고 있으며 빌드 단계는 없습니다. 변환에는
생성된 `pkg/` 출력만 필요하지만, 페이지 글꼴은 [DESIGN.md](DESIGN.md)에 기록된 Google Fonts 세
종을 원격으로 요청하고 문서화된 로컬 글꼴 스택으로 폴백합니다. 이 페이지는 순서가 보장된 진단
개수를 표시하고 개별 코드 항목으로 펼쳐 보여주며, 렌더러가 소유한 동작과 타이밍을 opaque origin
프레임에서 실행하고, 슬라이드 좌표를 유지한 채 이미지처럼 동작하는 전체 슬라이드 확대를 가용
너비에 맞춰 초기화합니다.

</details>

## 진단

생성된 모든 HTML 문서에는 순서가 보장된 진단이
`<script type="application/json" id="pptx2html-diagnostics">` 안에 JSON 배열로 포함됩니다. 폴백이
필요 없었던 변환에서는 `[]`가 들어갑니다. `unresolved_elements`는 SmartArt, OLE, Math,
사용자 정의 도형의 플레이스홀더 처리를 위한 호환용 투영으로 계속 제공됩니다.

렌더러가 스스로에게 부과한 규칙은 이렇습니다. 정확히 해석할 수 없는 요소는 원본을 그대로 보존하고
타입이 부여된 코드를 발행하며, 절대로 임의의 근삿값으로 뭉개지 않습니다. 해석되지 않은 사용자 정의
가이드 수식은 0으로 대체하지 않고 원본 수식을 그대로 유지한 채
`DRAWINGML_CUSTOM_GEOMETRY_FALLBACK`을 발행하고, 알 수 없는 채우기 패턴은 단색을 지어내지 않고
`DRAWINGML_PATTERN_UNSUPPORTED`를 발행합니다.

## 렌더러가 해석하는 범위

전체 ECMA-376 요소 목록은 [SUPPORTED_FEATURES.md](SUPPORTED_FEATURES.md),
지원 단계에 대한 기준 표는
[docs/architecture/CAPABILITY_MATRIX.md](docs/architecture/CAPABILITY_MATRIX.md),
현재 기능 원장과 남은 정확도 작업은
[docs/architecture/PPTX_COMPLETENESS_PROGRESS.md](docs/architecture/PPTX_COMPLETENESS_PROGRESS.md)를
참고하세요.

자동 생성 레지스트리는 추적 중인 56개 기능을 모두 포함합니다. 각 행은 의미·시각·동작 상태
(`S/V/B`)를 표시합니다. `exact`는 PowerPoint 네이티브 증거가 있어야 하며, `approximate`는
구현되어 있지만 알려진 차이가 있고, `fallback`은 완전한 렌더링을 주장하지 않은 채 제한된
콘텐츠나 진단을 보존한다는 뜻입니다. 아래 자동 생성 요약이 현재 개수이며 지금 `exact`인 차원은
없습니다.

| 분류 | 주요 내용 |
|----------|-----------|
| 도형 | 조정값을 폭넓게 지원하는 187개 프리셋 도형과 사용자 정의 도형 SVG 렌더링, 가이드 수식, 텍스트 사각형 |
| 텍스트 | 굵게, 기울임, 밑줄, 취소선, 위/아래 첨자, 세로쓰기, 형광펜, 그림자, 자간, 기본 18pt 폴백 |
| 색상 | RGB, 테마, 시스템, 프리셋 색상과 12종 수정자(tint, shade, lumMod, satMod 등) |
| 채우기 | 단색, 그라데이션, 이미지, 채우기 없음 렌더링. 패턴 채우기는 fallback 등급이며 정확한 지원을 주장하지 않고 미해석 원본과 진단을 보존. 스타일 참조(fillRef/lnRef) 포함 |
| 표 | 직접 셀 채우기, 텍스트/테두리, 행·열 병합. 패키지 표 스타일은 정의나 구성 요소를 해석할 수 없을 때 fallback 등급으로 보존·진단 |
| 이미지 | Base64 임베딩, `images/slide-N/` 아래 결정적 외부 자산, 자르기, MIME 자동 판별 |
| 레이아웃 | 마스터/레이아웃 상속, ClrMap 재정의, 플레이스홀더 매칭, TxStyles, bodyPr 속성 승계(줄바꿈, 여백, 세로 정렬, 세로쓰기, 자동 맞춤) |
| 글머리 기호 | 문자·자동 번호 글머리 기호와 슬라이드 문단·슬라이드 소유 목록 스타일·표 셀에 포함된 그림 글머리 기호 |
| 차트 | 간격/겹침과 1차 데이터 레이블을 포함한 묶은/누적/100% 누적 막대·세로 막대 직접 렌더링, 지점 레이블과 명시적 표식을 지원하는 단순 꺾은선/기본 영역/분산형, 단일 계열 방사형, 축 제목, 단일 계열 원형/도넛과 평면으로 렌더링되는 단일 계열 pie3D·area3D |
| 미디어 | 도형이 소유한 내부 PCM WAV와 결정적 단일 프레임 Constrained Baseline AVC MP4 부분집합에 대한 제한적 재생. 네이티브 컨트롤을 사용하며 자동 재생은 하지 않고 사용자 제스처를 요구합니다 |
| 노트·메모 | 슬라이드 노트, 노트 마스터 연결, 레거시 및 최신 메모/작성자를 타입이 부여된 화면 밖 진단 메타데이터로 보존 |
| 동작 | 도형, 그룹, 그림, 연결선, 표 그래픽 프레임, 텍스트 런 표면에 대한 클릭·마우스 오버 동작을 타입으로 보존 |
| 미지원 | SmartArt, OLE, Math — 메타데이터 사이드밴드(원본 XML, 타입, 위치)를 갖는 구조화된 플레이스홀더 |
| LLM 보강 | 후처리 계층: SmartArt→HTML/CSS, OMML→MathML, DrawingML→CSS (pptx2html-enhance) |

`mathPlus`처럼 이름이 `math`로 시작하는 DrawingML 프리셋은 기하 도형일 뿐이며 OMML 수식 지원을
의미하지 않습니다.

`[교차검증 필요]` 매니페스트의 `table-style`과 `slide-synchronization` 항목은 소스 검증이
완료되지 않았습니다.
현재 행은 저장소 구현 증거를 설명할 뿐, 독립적인 공식 소스 확인을 의미하지 않습니다. 호환성을
주장하기 전 각 매니페스트 행을 확인해야 합니다.

## 아키텍처

```
PPTX (ZIP) → parser/ (SAX XML) → model/ (Rust 구조체) → resolver/ (상속) → renderer/ (HTML/CSS)
```

속성 상속은 정해진 순서를 따릅니다: 슬라이드 도형 → 레이아웃 플레이스홀더 → 마스터 플레이스홀더
→ txStyles → defaultTextStyle → 명세 기본값. 색상은 슬라이드 색상 맵을 거쳐 테마에서 해석됩니다.

```
PPTX → pptx2html-turbo (Rust) → HTML + 메타데이터
                                    │
                                    ├─→ 직접 HTML 출력 (의존성 없음)
                                    └─→ pptx2html-enhance (Python, LLM) → 보강된 HTML
                                              │
                                              ├── SmartArt XML   → HTML/CSS 레이아웃
                                              ├── OMML 수식      → MathML
                                              └── DrawingML 효과 → CSS (그림자, 광선, 흐림)
```

전체 파이프라인 다이어그램과 모듈 책임은 [ARCHITECTURE.md](ARCHITECTURE.md)를, EMU 좌표계, 색상
해석 체인, 프리셋 도형 카탈로그, 플레이스홀더 타입, 슬라이드 상속은
[docs/reference/](docs/reference/)를 참고하세요.

<details>
<summary><b>보존 범위와 보안 경계</b></summary>

패키지 파서는 64 MiB보다 큰 입력, 항목이 8,192개를 넘는 압축 파일, 선언된 압축 해제 데이터가
256 MiB를 넘는 파일, XML 누계가 64 MiB를 넘는 파일, 개별 XML 파트가 16 MiB를 넘는 파일을
거부합니다. 브라우저 데모와 npm 편의 API는 `Blob`을 메모리로 읽기 전에 64 MiB 입력 제한을
적용합니다.

Rust 코어는 슬라이드 전환/타이밍 XML을 순서대로 보존하고, 상호작용 기반의 제한된 부분집합을
근사 실행합니다. cut/fade 전환과 해석된 슬라이드 도형에 대한 클릭/이전 효과와 동시/이전 효과 다음
나타내기·사라지기·페이드 효과가 여기 포함되며, 시작 조건 지연은 유한한 값에 한해 10000 ms까지
지원합니다. 자동 재생이나 반복은 하지 않습니다. 지원하지 않는 타이밍은 원본 노드 XML을 그대로
담은 타입 폴백 메타데이터로 남고, 지원하지 않는 대상은 정적으로 계속 보이는 상태를 유지합니다.
타이밍 인벤토리는 변환 내부에만 존재하므로 v2.0 이전의 공개 `Slide`, 기능 열거형, `TimingInventory`
API가 그대로 유지됩니다.

패키지 수준에서 지원하지 않는 파트와 관계는 눈에 보이는 도형을 만들지 않더라도 보고됩니다. 관계
진단은 원본 파트와 관계 ID만 식별하며 대상은 절대 노출하지 않습니다.

슬라이드 노트와 레거시·최신 메모는 내부 관계 파트에서 파싱되며 눈에 보이는 `.slide` 하위 트리
바깥에 남습니다. 문단 단위 텍스트, 1부터 시작하는 프레젠테이션 슬라이드 연결, 작성자 레코드,
타임스탬프, 관계 ID, 파트 이름, 검증된 노트 마스터 관계는 순서가 보장된 진단 JSON에
`fallback/parsed` 메타데이터로 실려 나갑니다. 작성자가 없다고 해서 메모 텍스트를 버리지 않고
`COMMENT_AUTHOR_UNRESOLVED`를 발행하며, 작성자 ID가 중복되면 임의의 레코드를 고르지 않고 미해석
상태로 둡니다. 안전하지 않거나 외부·잘못된 형식·중복·타입이 위조된 주석 관계는 무관한 패키지
파트를 선택하는 일이 없습니다. 알 수 없는 최신 메모 확장 하위 트리는 각각 독립적으로
`MODERN_COMMENT_EXTENSION_FALLBACK`과 함께 보존되며 정확한 해석이라고 주장하지 않습니다. 이
경계는 Microsoft의
[노트 슬라이드](https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-notes-slides),
[레거시 메모](https://learn.microsoft.com/en-us/office/open-xml/presentation/working-with-comments),
[PresentationML 구조](https://learn.microsoft.com/en-us/office/open-xml/presentation/structure-of-a-presentationml-document),
[최신 CT_Comment](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-pptx/161bc2c9-98fc-46b7-852b-ba7ee77e2e54),
[최신 Comment Part](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-pptx/b85a9293-bdca-4c6b-a554-8f3918db9791),
[최신 Author Part](https://learn.microsoft.com/en-us/openspecs/office_standards/ms-pptx/4071f53f-9509-405f-a76b-594b865e177a)
문서를 따릅니다.

패키지에 정의된 표 스타일은 문서화된 Office 영역 순서를 따릅니다. 변환기 정의를 사용할 수 없는
유효한 Office 기본 스타일이나 유효하지 않은 ID는 ID와 여섯 개 플래그를 보존한 채 외형을 지어내지
않고 `TABLE_STYLE_DEFINITION_UNAVAILABLE`을 발행합니다. 표 스타일의 `fillRef`는 인덱스, 참조 색상,
수정자를 보존하며 참조된 채우기가 파싱된 테마 형식 구성표에 존재할 때만 해석됩니다. 현재 테마
파서가 담을 수 없는 비단색 테마 채우기는 적용하지 않은 채 `TABLE_STYLE_PRIMITIVE_UNSUPPORTED`를
남기며, 이를 단색으로 뭉개는 것은 금지되어 있습니다. 범위가 지정된 `tblBg/effectRef`와 테두리별
`lnRef`의 인덱스/색상/수정자 역시 형제 스타일을 버리거나 효과·선을 지어내지 않고 보존한 뒤
미지원으로 진단됩니다. 머리글/바닥글 기준 행·열 밴드 원점은 근사값이며 PowerPoint와 동일하다고
주장하지 않습니다.

`Shape::actions`와 `TextRun::actions`가 타입이 부여된 동작에 대한 기준 계약입니다. 클릭과 마우스
오버를 구분하고 외부 URI, 실제 표시 순서상의 슬라이드 대상, 다음/이전/처음/마지막, 무동작, 미디어,
지원하지 않는 원본 동작 의미를 보존합니다. `TextRun::hyperlink`는 호환용 투영으로 남지만, 타입
링크와 레거시 링크 모두 동일한 제품 보안 정책을 통과해야 합니다. ASCII 제어 문자와 공백이 없는
`http`, `https`, `mailto` URI만 실행 가능합니다. HTTP(S) 자격 증명이 포함된 주소와 잘못된 형식,
상대 경로, 프로토콜 상대 경로, 파일, 프로그램, 매크로, 사용자 정의 대상은 모두 비활성입니다.
실행 가능한 외부 링크는 `rel="noopener noreferrer"`와 함께 새 브라우징 컨텍스트에서 열리며, 마우스
오버 메타데이터는 절대 이동을 일으키지 않습니다. 경계 및 숨김 슬라이드 이동은 근사값이며
PowerPoint와 동일하다고 주장하지 않습니다.

그룹과 표 그래픽 프레임 동작은 하위 요소의 동작을 덮어쓰지 않고 자신의 `cNvPr` 정체성을
유지합니다. 타입이 부여된 런과 안전한 레거시 `TextRun::hyperlink` 앵커는 이를 감싸는 도형, 그룹,
표 동작 표면보다 위에서 포인터에 도달할 수 있습니다. 일반 런과 차단된 안전하지 않은 레거시 링크는
소유자 동작을 가로채지 않습니다. 런 진단은 안정적인 슬라이드/도형/문단/런 좌표를 사용하고, 해당하는
경우 표의 행·열 좌표를 함께 사용합니다. 완전히 동일한 발행은 합쳐지고 서로 다른 발생은 분리된 채
유지됩니다. 동작 파싱에는 정확한 PresentationML 소유자/비시각/`cNvPr` 스택과 DrawingML 동작
네임스페이스가 필요합니다.

도형이 소유한 `a:audioFile`과 `a:videoFile` 참조는 `ppt/media/` 안으로 안전하게 해석되고 16 MiB
이내이며 네임스페이스가 유효한 콘텐츠 타입을 가진 공식 내부 관계에 한해 지원됩니다. 오디오는 PCM
WAV로 제한됩니다. 비디오는 8비트 4:2:0 프로그레시브 Constrained Baseline AVC(프로파일 66, 호환성
`0xc0`, 레벨 30)의 구조적으로 파싱된 IDR I 슬라이스 하나로 제한되며, 16x16부터 256x256까지
매크로블록에 정렬된 크기, 래스터 순서의 I_PCM 매크로블록, 표준 emulation prevention 및 trailing
bits, `avc1`/SPS 크기 일치, `mdat` 내부에 완전히 포함된 샘플 테이블 범위를 요구합니다. 추가
파라미터 세트, 슬라이스, NAL 유닛이나 지원하지 않는 AVC 문법은 폴백 처리되며, 픽스처 바이트나
픽셀 바이트 화이트리스트는 인정하지 않습니다. 지원되는 자산은 자동 재생 없이 네이티브 컨트롤을
사용하고, 외부 관계는 절대 가져오지 않습니다. 브라우저 코덱 동작과 PowerPoint 원본 충실도는
근사값이며 정확하다고 주장하지 않습니다.

</details>

<details>
<summary><b>v2.0.0 API 호환성</b></summary>

v1.x에서 올라오는 Rust 사용자는 다음 공개 API 변경을 반영해야 합니다.

- `Bullet::Picture`가 새로운 exhaustive match 갈래를 추가합니다.
- `Shape::actions`, `TextRun::actions`, 타입이 부여된 동작 열거형, `FallbackKind::ActionMetadata`가 동작 처리를 확장합니다.
- `TableStyleReference::definition`이 `Option<Box<TableStyle>>`로 바뀌었고, 공개 표 스타일 구조체에 타입이 부여된 필드가 추가되었습니다.
- `Presentation::embedded_inventory` 공개 필드가 추가되었습니다. 필요한 곳에 `..Default::default()`를 사용하세요.
- `ConversionResult::diagnostics` 공개 필드가 추가되었습니다. 결과는 `ConversionResult::new(html, slide_count)`로 생성하세요.

`diagnostics` 필드 때문에 기존 외부 구조체 리터럴은 소스 호환성이 깨지지만, 기존
`unresolved_elements` 필드와 그 반환 투영은 그대로 유지됩니다. `ConversionResult::diagnostics()`가
안정적인 순서 슬라이스 접근자를 제공합니다. v2.0.0 표 스타일 모델은 공개 `TableData`,
`TableCell`, `TableCellStyle`, `TableStyle` 구조체에 타입이 부여된 미지원 참조 원시 타입을 포함한
필드를 추가하므로, 구조체 리터럴을 쓰는 외부 코드는 `..Default::default()`를 사용하거나 새 필드를
명시해 이전해야 합니다.

</details>

## 평가

충실도는 주장하지 않고 실제 렌더링된 기준 이미지와 비교해 점수를 냅니다. 종합 점수는
`0.40*SSIM + 0.25*TextMatch + 0.25*TestPassRate + 0.10*Performance`입니다. PowerPoint 네이티브
기준이 1차 충실도 기준이고, LibreOffice 기준은 2차 회귀 신호입니다. 종합 점수는 회귀 관리에
사용하되, 어떤 기능을 `exact`로 표시하기 전에는 반드시 PowerPoint 기준 검증을 거쳐야 합니다.

<details>
<summary><b>평가 하네스 실행</b></summary>

```bash
cd evaluate
python -m venv .venv && source .venv/bin/activate
pip install -r requirements.txt && playwright install chromium

# 1. 결정적 골든 픽스처 세트 생성
python create_golden_set.py

# 2. 기준 렌더링 (LibreOffice 헤드리스, 또는 Windows에서 PowerPoint)
python reference_render.py --input golden_set/ --output golden_references/
pwsh -File ./reference_render_powerpoint.ps1 -InputDir ./golden_set -OutputDir ./powerpoint_golden

# 3. 종합 충실도 점수 계산
python evaluate_fidelity.py --project-root ..
```

정확도 계약 검사기와 CI/릴리스 평가 워크플로가 공유하는 Python 3.11+ 하한을 포함한 자세한 내용은
[`evaluate/README.md`](evaluate/README.md)를 참고하세요.

</details>

<details>
<summary><b>7종 형식 합격 게이트</b></summary>

7종 형식 합격 게이트는 의도적으로 fail-closed로 동작합니다. 기본 필수 기준 프로필은
`libreoffice-poppler`입니다. 지원되는 macOS 호스트에서는 여섯 Office 형식에 LibreOffice와
Poppler를 사용하고 PDF에는 Poppler를 직접 사용해 바인딩된 기준 증거를 생성합니다. Linux에서도
문서 변환은 지원하지만 Linux 프로세스 샌드박스 백엔드가 구현되기 전까지 서명된 휴대형 기준 캡처는
`INCOMPLETE`입니다. 이 프로필은 고정된 7종 형식 코퍼스를 사용하며, 허용된 각 소스, 도구, 런타임,
출력마다 형식별 schema-2 휴대형 락, 서명된 영수증, SHA-256 바인딩을 요구합니다. 누락되거나 오래되었거나
대체 또는 변조된 증거는 `INCOMPLETE` 또는 `FAIL`로 남습니다.

승인된 일반 성능 표기는 `96% under the documented general conversion evaluation contract`입니다.
서명된 휴대형 프로필 한 웨이브가 7종 형식 전체에서 통과한 뒤에만 사용할 수 있습니다. 이 표기는
Microsoft Office 픽셀 정확도, PowerPoint 픽셀 일치, 바이트 단위 동일 출력, 또는 PPTX `exact` 등급을
의미하지 않습니다.

```bash
uv run python -m evaluate.multiformat_gate \
  --reports-dir evaluate/multiformat/reports \
  --oracle-lock-dir evaluate/multiformat/oracle-locks
```

락 디렉터리에는 필요한 각 형식의 schema-2 락인 `pptx.json`, `docx.json`, `doc.json`,
`xlsx.json`, `xls.json`, `ppt.json`, `pdf.json`이 정확히 하나씩 있어야 합니다. 하나의 공유 락으로
이 집합을 대체할 수 없습니다.

코퍼스 매니페스트는 후보 실행 전에 독립적으로 검증합니다.

```bash
uv run python -m evaluate.multiformat_corpus \
  --manifest evaluate/multiformat/wave/corpora/docx/manifest.json
```

원시 후보/기준 아티팩트는 `python -m evaluate.assemble_multiformat_report`로 결정적으로 채점·조립
됩니다. 제품 게이트는 동일한 보고서를 다시 계산하며 손으로 편집한 집계는 거부합니다. 보안 코퍼스
라벨은 실행 전에 OOXML, CFBF, PDF 픽스처 구조에서 독립적으로 도출되며, 해시와 기대 결과 라벨만으로는
보안 하드 게이트를 통과할 수 없습니다. 네트워크가 격리된 2회 실행 Chromium 후보는
`python -m evaluate.capture_multiformat_candidates`로 생성합니다. 고정된 런타임과 샌드박스 계약은
`evaluate/README.md`를 참고하세요.

서명된 Microsoft Office/Windows 오라클 증거는 선택적 `microsoft-office` 프로필로 계속 지원합니다.
기본 휴대형 합격 경로의 필수 조건은 아닙니다. 이 프로필을 선택하면 schema-1 락, 서명된 영수증,
검증 키에 바인딩된 캡처, 출처 정보, 아티팩트 해시가 모두 fail-closed로 검증되어야 합니다. 누락되거나
유효하지 않은 Office 증거를 휴대형 증거로 바꾸어 표시하거나, 통과한 Office 프로필 웨이브로 만들 수
없습니다.

</details>

<details>
<summary><b>Autoresearch 실험 루프</b></summary>

[Karpathy의 autoresearch](https://x.com/karpathy/status/1886192184808149383) 패턴에서 착안한 자동
실험 루프입니다. LLM 에이전트가 소스를 수정하고 빌드/테스트/평가를 실행한 뒤, 충실도 점수가
올라간 경우에만 변경을 유지하고 그렇지 않으면 되돌립니다.

```bash
# 특정 단계 실행
./autoresearch/run_loop.sh --phase 01_color_fidelity

# 반복 횟수 제한
./autoresearch/run_loop.sh --phase 02_performance --max-iterations 50
```

| 단계 | 목표 |
|---|---|
| `01_color_fidelity` | 테마 색상 수정자 정확도 (12종) |
| `02_performance` | 렌더링 처리량 최적화 |
| `03_effect_rendering` | 그림자/광선 DrawingML → CSS 변환 |
| `04_geometry_coverage` | 프리셋 도형 확장 (30 → 187) |

결과는 `autoresearch/results.tsv`에 기록됩니다. 전체 프로토콜은 `autoresearch/program.md`를
참고하세요.

</details>

<details>
<summary><b>pptx2html-enhance (LLM 후처리)</b></summary>

Rust 변환기의 출력을 LLM 제공자로 보강하는 선택적 Python 패키지입니다. 구조화된
플레이스홀더(SmartArt, Math, OLE)를 의미 있는 HTML로 대체합니다.

```bash
pip install ./pptx2html-enhance[anthropic]   # 또는 [openai], [all]
```

```python
import pptx2html
from pptx2html_enhance import enhance

result = pptx2html.convert_with_metadata("presentation.pptx")

enhanced_html = await enhance(
    result.html,
    [e.__dict__ for e in result.unresolved_elements],
    provider="anthropic",       # 또는 "openai"
    timeout=30.0,
    max_concurrent=5,
)
```

| 타입 | 핸들러 | 처리 방식 |
|---|---|---|
| SmartArt | `SmartArtHandler` | LLM이 원본 DrawingML XML을 HTML/CSS 레이아웃으로 변환 |
| Math (OMML) | `MathHandler` | 간단한 수식(분수, 첨자, 근호)은 규칙 기반, 복잡한 수식은 LLM 폴백 |
| 효과 | `EffectsHandler` | 규칙 기반: 바깥 그림자 → `box-shadow`, 광선 → `box-shadow`, 부드러운 가장자리 → `filter: blur()` |

</details>

## v2.1.0 배포 상태

[v2.1.0](https://github.com/brnyxx/pptx2html-turbo/releases/tag/v2.1.0)은 게시되었습니다.
`@briank-dev/pptx-to-html@2.1.0`, `@briank-dev/pptx2html-turbo@2.1.0`, 플랫폼별 GitHub Release
아카이브와 [GitHub Pages 변환기](https://brnyxx.github.io/pptx2html-turbo/)를 사용할 수 있습니다.

[v2.1.0 검증 및 게시 보고서](docs/release-notes/v2.1.0-validation.md)는 리뷰된 소스 경계,
재현 가능한 게이트, 실제 패키지·브라우저 게시 영수증, 보존된 커밋 이력, exactness 제한을
기록합니다. 이 릴리스는 approximate/high-fidelity 릴리스이며 Microsoft Office 픽셀 정확도,
PowerPoint 픽셀 일치, 바이트 단위 동일 출력 또는 PPTX `exact` 등급을 주장하지 않습니다.

GitHub Pages 데모의 시각 토큰, 컴포넌트, 반응형 규칙, 모션, 수용된 기술 부채는
[DESIGN.md](DESIGN.md)에 정리되어 있습니다.

## 개발

```bash
cargo test --workspace                                        # Rust 테스트
cargo bench --package pptx2html-core                          # 벤치마크
cd pptx2html-enhance && .venv/bin/python -m pytest tests/ -v  # Python 테스트
python3 -m unittest discover -s evaluate/tests -p 'test_*.py' -v
```

<details>
<summary><b>프로젝트 구조</b></summary>

```
├── crates/
│   ├── pptx2html-core/        # 코어 라이브러리 (model, parser, resolver, renderer)
│   ├── pptx2html-cli/         # CLI 바이너리 (clap)
│   ├── pptx2html-py/          # PyO3 Python 바인딩 (maturin)
│   ├── pptx2html-wasm/        # WASM 바인딩 (wasm-bindgen) + 데모 페이지
│   ├── document2html-core/    # 형식 중립 엔진 + 형식 판별
│   ├── document2html-native/  # LibreOffice/Poppler 네이티브 파이프라인
│   ├── document2html-py/      # 통합 문서 Python 바인딩
│   └── document2html-wasm/    # 통합 문서 브라우저 WASM 패키지
├── evaluate/                   # 충실도 평가 (건드리지 말 것)
├── autoresearch/               # Autoresearch 실험 루프
├── pptx2html-enhance/          # LLM 후처리 (Python)
└── docs/                       # 아키텍처, 레퍼런스, 릴리스 노트
```

</details>

## 전체 PPTX 기능 레지스트리 (자동 생성)

아래 표는 `evaluate/completeness_manifest.json`에서 자동 생성되며 정확도 계약 검사기가 영문
README 및 아키텍처 문서와 함께 동기화합니다. `Feature`는 기능 ID이고 `Current S/V/B`는 현재
의미·시각·동작 상태, `Target S/V/B`는 검증된 목표 상태입니다.

<!-- BEGIN GENERATED PPTX CAPABILITY MATRIX -->
<!-- manifest-sha256: dd24142f66dbd737b6ef27f77ac4bc433053bc1249e86965c34033a19b32da47 -->
<!-- current-tier-counts: exact=0 approximate=54 fallback=114 unparsed=0 -->
Current disposition totals: **0 exact**, **54 approximate**, **114 fallback**, and **0 unparsed** across 56 features and three dimensions.

| Feature | Current S/V/B | Target S/V/B | Verification SHA256 | Status SHA256 |
|---|---|---|---|---|
| <a id="capability-presentation"></a>`presentation` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `c07e2810b8d5e13a63436f7b11c3ee961e11b15f61bdc50a1ca260c0738e4a4f` | `29665c44b1b28428449e05099e8b3f5d22f1e577d8eaaf700a7f1c9a1b347de5` |
| <a id="capability-presentation-properties"></a>`presentation-properties` | approximate/parsed<br>fallback/not-applicable<br>fallback/not-applicable | approximate/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `03b3697960c6db57bc2d101452d5e8abc0a9ecd7ed2048d867a97032ccb94e5b` | `cf3d3cadc4899f4321326655a859005131cd42d60dc1e24accad86220543b42d` |
| <a id="capability-slide-master"></a>`slide-master` | approximate/resolved<br>approximate/rendered<br>fallback/not-applicable | approximate/resolved<br>approximate/rendered<br>fallback/not-applicable | `d26c42cad024a240ba42584139d32b0485d45f86a946ebc65d2cf2c2d9c920eb` | `2fcbe53ce1225a110400f235335397da53ab763ef52d242204931561cf098958` |
| <a id="capability-slide-layout"></a>`slide-layout` | approximate/resolved<br>approximate/rendered<br>fallback/not-applicable | approximate/resolved<br>approximate/rendered<br>fallback/not-applicable | `80a9fec92635d749ef0271cfb91e56a7c2b642a42f42a3719badde4160d0e329` | `fd2002a3e42946c1a1212cdb072c36fdc16f6aa2f56c1c6ae6920649413f4792` |
| <a id="capability-slide"></a>`slide` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `d7216600198cf446aa21948013131473434b57228017fdd7c2eea16a3aee2ed7` | `9ed1789d738b9c6f29e7712866cb1b72ef0b9798f5f78d5e3210d92d59eeaf4c` |
| <a id="capability-theme"></a>`theme` | approximate/resolved<br>approximate/rendered<br>fallback/not-applicable | approximate/resolved<br>approximate/rendered<br>fallback/not-applicable | `a1050e25c09f1b3687932cd923ac2c5e9ac8b8bd04ea694e1af75f7ff6397807` | `70df65b760e43407d76fcadcbc3fb5e52fe68c9cd94624c584352cc2bffb0921` |
| <a id="capability-notes-master"></a>`notes-master` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `7c0f6c034617ee80dfedda6fad705b98bd052084f09a7878d8f44c0b8637b507` | `f2dcd5a888468034bfcb5e696a84f70f017ab138c1727937b79cbbd743f21e3a` |
| <a id="capability-notes"></a>`notes` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `1b5af7f5ec83a70268e65aa5017a47d559c69452cea72f455c343edd4ac94e51` | `1e0e297d3d1c8e823ed852c6eb690944605bbf290c62c24bf39300d901642b7f` |
| <a id="capability-handout-master"></a>`handout-master` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `891c69a9b73e211f98dba58561eff7c132fccbe56f73cc738a94d39aa81c3b4a` | `9d44cff55da2c0159e8c5dcc8ead0ff6e9769ead1dd7e6e0c3efaabb2b811497` |
| <a id="capability-comments"></a>`comments` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `944fa74d1b1a1aec97d94eee1d54feb252a2b139a54939ff9388ded6595591b9` | `2ea3f2aafdfa77fd66c34f43fb85bfb4f993bf50cba40edee5eda1165d8340e9` |
| <a id="capability-comment-authors"></a>`comment-authors` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `3fdf939a544a498dda287c4cbec1ef75ccfbe8b3f5aa080ef114614b91d7900a` | `85ea90cb75643a556bd9dba65f0ce49610b7ff62b985d3ea8636f6cfbaa3ed1b` |
| <a id="capability-shape-tree"></a>`shape-tree` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `039ce2b4d821932f9c2243102b5c97dbcd41d0f4ecfc0f7e01b0fde941e7805e` | `3f86cff8d830a06e21d3779e44a9b21194756e2ad8955aefdeaba3fc9db1162a` |
| <a id="capability-preset-shape"></a>`preset-shape` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `cb84ad4e1f0ca5b1849c7a3331a9a878a3d0b3818352f158c405e19c87a88fd2` | `5d446d085d5c42ea91cc6540d5b83bbfaca15e62afe42e6f9c20d4d59ea9a86f` |
| <a id="capability-custom-geometry"></a>`custom-geometry` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `244b8537a5f7fd49e3fafd5a462a12d5f6cf0408a8cf3235e7645b0baefea8f5` | `99c76b2c42fdf8b00e68efc337816db612d39bb09426e39028af8db8b1051083` |
| <a id="capability-connector"></a>`connector` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `7e7b1b3a0a60e49d6702574dba2a1929d3e4c82abd8f7b60a7d162a0f63fa509` | `f469f88311b3de633ad23f2d8257cd92e2faaa75299ee824ac1279ee1f00367c` |
| <a id="capability-group-shape"></a>`group-shape` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `393472e96359637a79aa7a838f6c16db5b9d71b24cb648fefea81e3a646a41fb` | `e5f16afa6c7699ece99d11402306f0119f415730b8889499312d6be6083db36e` |
| <a id="capability-picture"></a>`picture` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `dffb48ca4b06c68069e0b407c9934ceaceb8dabf447bedac71f10b581a2ac645` | `7199c2265f56c189e0b25a8f38529f37da9174155adb2c46b2e236d3105947f8` |
| <a id="capability-text-body"></a>`text-body` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `262f4fb2d080594a9c78a70b702253e646af04a1e7e86f2d9b8debfe18f15e8b` | `bbbb778196c659c4ba3931d9f51c8383575a005812fde7c4f92a85d90cf53e89` |
| <a id="capability-rtl-text"></a>`rtl-text` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `c503fb10524fa65e82d1d4ea5d4de2579f51949547d1de8ad5cb1b496f0070e5` | `85173066116d7250da3058a7f80b43b147cfeef918f4cf802bbf94dff3613c65` |
| <a id="capability-bullets"></a>`bullets` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `ed157a688196eea774392c88ae5db59cb6cfc0f7167532360488ca899ebdff3d` | `7083d9593322381b21f9ac938277da2637c57b8e9663fe7baf886efe289ff341` |
| <a id="capability-picture-bullets"></a>`picture-bullets` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `7d1c21ce2540da7b56a5a48196f9f4d69d56c985e23afd6772a5b96d1de5508f` | `d4d97387d415bb350ee62522151319c7190d7a60f9fc6a33ad16fd2953d680d0` |
| <a id="capability-fills"></a>`fills` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `f7a7e6203cadf6138eda6a0262ea7f8413a200044cbaa8be71445d6ee0d08e7b` | `27f0d1439c068d3dcdc802df5c98749ad63a526753bf4e411cd97c0a5025cac2` |
| <a id="capability-pattern-fill"></a>`pattern-fill` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `aa65f7e14d906cfa690b48408c5e59168b09e5ec7f29366695a335778beb8fab` | `e7687dc0b1523f4d8d835a27538507091664a2af8daf52c7cefa2253b28a7171` |
| <a id="capability-effects"></a>`effects` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `768bd8e0e131deaf5a963f37a66952f4287ebff4a860cf8d2fda726f2f67968d` | `7e79f784844b8576e35fa68dce69588d336125fab6a6e84caf40373b91880b73` |
| <a id="capability-reflection-and-3d"></a>`reflection-and-3d` | fallback/parsed<br>approximate/rendered<br>fallback/not-applicable | fallback/parsed<br>approximate/rendered<br>fallback/not-applicable | `05625623d02d2afb0f7c3529951fd70e1f3611f7ba5acacb447b5e512abac08d` | `22d06b2ad85fb0a25de923ea582f347b688efbb1330a7e110660f45afea9c183` |
| <a id="capability-table"></a>`table` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `4e5951bb9a4549790b7adc79890517a1225009b40688d246c11850c66101d192` | `7ae399ecfa572df16f042587cb995cdb8c754fbf48cc584ff6c28c79083e8d3b` |
| <a id="capability-table-style"></a>`table-style` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `e87d1531fdab2c0c063de4a617627c411454f05c6359e2b93c499fed5638617e` | `8507e8b5258344ccbf42786395cd9e9c1305007d9abc67292710353c91254cce` |
| <a id="capability-image"></a>`image` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `66ee0ff62f62adf90b2cb61bd3298f76d6db7d7e54e03632ffd5ff38e026714a` | `cf16268eadaa17f2829467c88b11c2858d7c58fd445c2c45d803d7b38ac8c213` |
| <a id="capability-chart-direct-subset"></a>`chart-direct-subset` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `9f1b331a89dc0443e3f4a31837f1ab9da612a9570c789fd9dd8e0503e9600643` | `377a904a5d76d39a2ba0164bfcaa24fe1b451c01555b70940225ffd655df7287` |
| <a id="capability-chart-preview-fallback"></a>`chart-preview-fallback` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `9b15b4f2cefcc9a46086fd4b54264d753f9e874554b4e153c0e4f8f5fb15ea29` | `587a7fd372d58f5da936b784d45cbdfa7536d5c3a5a95d31d5274264c8dc0c73` |
| <a id="capability-chart-placeholder-fallback"></a>`chart-placeholder-fallback` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `5f0b4fdecb60710becd532d16762d18734667afea2cda8d28449a5f25da1f9ad` | `4d57460e0f8ebae9e2e593c40d9876782b2e0bb6cfd1dfb8eb6d8e9730b8d49b` |
| <a id="capability-diagram"></a>`diagram` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `0cecd24eb6161f5bad365f66ddf4877732436c3ca3e0e67dfb2a76475572cf3b` | `2bb9eca9b9fd5342b7090b50836f0832acfe59b7d877dd77a8a172efcd3d2e0b` |
| <a id="capability-diagram-data"></a>`diagram-data` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `33d058f921ab4bf96eb875079f95b5c6a103dfd9fbb60ecb5c6b54684882aa19` | `e63c3b734b25079b0df064d2f74f4f085d4d8e6b345afb3b04b45c6f639625fb` |
| <a id="capability-diagram-layout"></a>`diagram-layout` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `39729d3ac8e6afc2d55966c8170f8fbc9412921364b9c031faf980945f9e08fb` | `5c71485b56affe554eccbc54e7c24d5f8d267033dc34b95065fe6ddab4da9427` |
| <a id="capability-diagram-styles"></a>`diagram-styles` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `0dce3209140ee3800b43953d43b6d77dd727cbe19ea1699d088bf2ffccee8725` | `e5cfd249fd43693753b370d54b8846c9eb397e0583fb734ef665c394db77ee19` |
| <a id="capability-diagram-colors"></a>`diagram-colors` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `835995fb45ba39bcffc948c66a1714647a4dde4f45b860bbf04c6e32918dc681` | `c8048fc748ccbf5216d5c9b3e55fcef0ac3fcd062ca75ee75f10378a49429032` |
| <a id="capability-ole-embedded-object"></a>`ole-embedded-object` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `9a1ea008d8a2422d170624f54c315e1e1ff435dee7a9f7528ab130827840486b` | `03972ca8681ad5adfff52f278be1c4c35b0ebaf19251d75f88b1f4eed8a04cc6` |
| <a id="capability-math"></a>`math` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `f902b654054d7ac1aaea679b5832d73bbb121c6d14a593df127bb97a77df9dbd` | `98676a3ba2f695ae7b3fc77b29d51d0b65cc21c8cdbe976aa61777b5637c29c6` |
| <a id="capability-media-audio"></a>`media-audio` | approximate/parsed<br>approximate/rendered<br>approximate/rendered | approximate/parsed<br>approximate/rendered<br>approximate/rendered | `72f9f2545ef7b485e028296680e9943b5b679f55ec7bfc267a4659fa459c2bdb` | `115a7ac4ad92809c52144bca695530c20c42876eb4cd62a92903a793721370ef` |
| <a id="capability-media-video"></a>`media-video` | approximate/parsed<br>approximate/rendered<br>approximate/rendered | approximate/parsed<br>approximate/rendered<br>approximate/rendered | `2de9f9aa1ac20fdda24dff34d3317856b28bc00dcaee216df808cee57158ae08` | `55c5b1bd4d7d05b9e7f5297572607be5fd9e1607eb98bec331ea411c041b83db` |
| <a id="capability-hyperlink-run-and-cell"></a>`hyperlink-run-and-cell` | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | approximate/parsed<br>approximate/rendered<br>fallback/not-applicable | `8078ebcf0df8602a6acf21547e7e42a8ade526d127fd7e921d249ae07b88d993` | `57dc2d2d733cbce264d1b225496048d1d95072ddb15fce3c38f4b8728124983b` |
| <a id="capability-shape-hyperlink-and-action"></a>`shape-hyperlink-and-action` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `d240b3956c52dba4526750cddd4d9c7a2690f59c295075997f6e3bb46b71664f` | `db947d63b09b3d26c18ff44bc685731344501a9beb14ad530512501e04230603` |
| <a id="capability-timing-and-animation"></a>`timing-and-animation` | approximate/parsed<br>approximate/rendered<br>approximate/rendered | approximate/parsed<br>approximate/rendered<br>approximate/rendered | `ee976c5f050029d337e0ea3a1ff5cfe3351b9aa59f3da5042e507eeaecfa521f` | `30e10705c96190b94004219490a97a7116fcf5f49a9c0b45ca5730fe39f1ce35` |
| <a id="capability-transitions"></a>`transitions` | approximate/parsed<br>approximate/rendered<br>approximate/rendered | approximate/parsed<br>approximate/rendered<br>approximate/rendered | `e06c8a2724ec2b5c11b4f4fbea9c88c66d4957fa756d15c2d27a543f6cf6719c` | `bdc5bc99fe9a448365a3d9721e6e67ca4df2fea07b674856df476f9aedfaef1c` |
| <a id="capability-extensions"></a>`extensions` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `4894bba77c5de06b7102b327fc78201befcc59a7b37cf9aa2f85c1f8e6ac0305` | `b36f463983b9b6f31f21ee7624b8179f3c336069e97235efd07f4c6933e6ad25` |
| <a id="capability-alternate-content"></a>`alternate-content` | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | fallback/parsed<br>fallback/rendered<br>fallback/not-applicable | `568918777892e84262c3bf521a5297a698db8831598d085a54cbf2840280c221` | `fb843b603490ab7412c7d1c34c18389bbfb9b5d8b973116d530064eec8caee18` |
| <a id="capability-bibliography"></a>`bibliography` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `e23a461817a9ded877cb7eb1e4979501178769765e246971ab74578a4ffe4ebb` | `30ae0425dec8aa78fc8c534d721be0277cce56b84761cfbdc4562175005a5f25` |
| <a id="capability-additional-characteristics"></a>`additional-characteristics` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `f10ed7446d28df9e489140d5c04044a23d86d782cdcfad33eaf6fb000fc8aaf2` | `3be632f7c8a7c60cff5633dd014bdf1f7e036a8c6431adc7bec1e6b8ec3ab2af` |
| <a id="capability-custom-xml"></a>`custom-xml` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `6328918018db2ff76d4aa2d8c8b27bbdace4bc71d46fba6b764209026b2c94c6` | `0e27bb416ec6d01d306d50e4976418e0743916f7531705fd88f99aa855983008` |
| <a id="capability-thumbnail"></a>`thumbnail` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `eea1202e0937556ba322f690e25073981337ab75cb3f640432aed42981fb1a83` | `ac63bbea2b37bedfb131e943838539e1d7373e7a3686fea14f95ee8dfed820c3` |
| <a id="capability-theme-override"></a>`theme-override` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `a6c3565ab75b88f7bcd341876512c4752c266a017d1d6d5ba08aa37b5cda995d` | `8dbd8139836a153e1e69009efcb939bd980708c67e19e790aafba14bb2c71dc9` |
| <a id="capability-slide-synchronization"></a>`slide-synchronization` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `d251deca6b42414d070751e4e079abd3c75abebf6fb296bb9c61d48be6e604d1` | `7108f8d030277f501eccd5e01cfef2389496178cf6128c7ae5248a8b067d1d42` |
| <a id="capability-content-part"></a>`content-part` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `2f7dce33c2e6808355a43fe0820855450ade1abd9fffe83bf6989965dc3da5d9` | `a6dc798a71b64907ffa02c9c93548a78f91ec78b0fca9852ffa861abd11f649e` |
| <a id="capability-embedded-package"></a>`embedded-package` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `9c9a08d8fb4442f66df36bc3de23ca6a0d0448bab2260996ed41c262cca6d5c0` | `1027870090ccee53b686f31b5098514211c5799b534f317f604406e734c57627` |
| <a id="capability-embedded-control-persistence"></a>`embedded-control-persistence` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `8cf43f357e46ee3defd6250fa099d6f88a37f4ac976b58cb6e5c6898c1785ce2` | `9353e1d1789f94b67689440757d4617fa6f283426188298e1914fb12f0922f82` |
| <a id="capability-user-defined-tags"></a>`user-defined-tags` | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | fallback/parsed<br>fallback/not-applicable<br>fallback/not-applicable | `d2bd311c48e46b4ba4449d05eb1b99762d2cc782adb325ec275a07b84c29a6d7` | `c5d90044021cd20e3c67fe72a821ca0073e55a3dde89af7916abfc57ce31f26c` |
<!-- END GENERATED PPTX CAPABILITY MATRIX -->

## 기여

개발 환경 설정, 코드 스타일, 제출 지침은 [CONTRIBUTING.md](CONTRIBUTING.md)를 참고하세요.

## 라이선스

MIT - [LICENSE](LICENSE) 참고
