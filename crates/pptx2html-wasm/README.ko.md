<div align="center">

<img src="https://raw.githubusercontent.com/brnyxx/pptx2html-turbo/main/.github/assets/hero.svg" alt="pptx2html-turbo — 서버 없이 PPTX를 HTML로. WebAssembly로 컴파일되는 순수 Rust ECMA-376 렌더러." width="920">

[![npm](https://img.shields.io/npm/v/@briank-dev/pptx-to-html?style=flat-square&labelColor=0B0F0E&color=F2703C&label=npm)](https://www.npmjs.com/package/@briank-dev/pptx-to-html) [![Rust](https://img.shields.io/badge/rust-2024_edition-8FD9AE?style=flat-square&labelColor=0B0F0E)](https://github.com/brnyxx/pptx2html-turbo) [![WebAssembly](https://img.shields.io/badge/wasm-browser_ready-F0C24E?style=flat-square&labelColor=0B0F0E)](https://brnyxx.github.io/pptx2html-turbo/) [![License](https://img.shields.io/badge/license-MIT-ECF1EE?style=flat-square&labelColor=0B0F0E)](https://github.com/brnyxx/pptx2html-turbo/blob/main/LICENSE)

[English](https://github.com/brnyxx/pptx2html-turbo/blob/main/crates/pptx2html-wasm/README.md) · **한국어**

</div>

PowerPoint 문서를 **브라우저 안에서 전부** HTML로 변환합니다. 렌더러는
[ECMA-376](https://ecma-international.org/publications-and-standards/standards/ecma-376/)
표준을 보고 Rust로 처음부터 구현한 뒤 WebAssembly로 컴파일한 것입니다. 서버 왕복도, 업로드도,
PowerPoint나 LibreOffice 설치도 필요 없습니다. 파일은 탭을 떠나지 않습니다.

렌더러가 정확히 해석하지 못한 요소는 조용히 버리거나 임의로 대체하지 않고, 코드가 부여된
진단(diagnostic)으로 보고합니다.

**[라이브 데모 열기](https://brnyxx.github.io/pptx2html-turbo/)** — `.pptx` 파일을 끌어다 놓으면 바로 변환됩니다.

## 설치

```bash
npm install @briank-dev/pptx-to-html
```

## 사용법

편의 API는 첫 호출 시 WASM을 초기화하며 브라우저 `File`/`Blob`, `ArrayBuffer`, `Uint8Array`를
모두 받습니다.

```html
<iframe id="output" sandbox="allow-scripts" title="Converted slide output"></iframe>
<script type="module">
import { pptxToHtml } from '@briank-dev/pptx-to-html';

const html = await pptxToHtml(file);
document.getElementById('output').srcdoc = html;
</script>
```

> **보안:** 변환 결과물은 실행 가능한 신뢰할 수 없는 HTML입니다. 위 예시처럼 `allow-scripts`를 준
> 샌드박스 iframe에서 렌더링하고 `allow-same-origin`은 **절대 넣지 마세요**.

<details>
<summary><b>저수준 API — 명시적 초기화, 필터링, 메타데이터</b></summary>

```html
<script type="module">
import init, {
  convert,
  convert_with_options,
  convert_with_metadata,
  convert_with_options_metadata,
  get_presentation_info,
} from '@briank-dev/pptx-to-html';

await init();

const response = await fetch('/presentation.pptx');
const data = new Uint8Array(await response.arrayBuffer());

const html = convert(data);

const filtered = convert_with_options(
  data,
  true,
  false,
  new Uint32Array([1, 3]),
  1.0,
);

const info = get_presentation_info(data);
console.log(info.slideCount, info.widthPx, info.heightPx, info.title);

const withMetadata = convert_with_metadata(data);
console.log(JSON.parse(withMetadata.diagnostics));
console.log(withMetadata.unresolvedElements);

const filteredWithMetadata = convert_with_options_metadata(
  data,
  true,
  false,
  new Uint32Array([1, 3]),
  1.0,
);
</script>
```

</details>

## API

- `pptxToHtml(input, moduleOrPath?)` — WASM을 지연 초기화하고 브라우저 입력이나 바이트를 변환합니다. 동시 호출은 첫 초기화 시도와 그 성공/실패를 공유하며, 실패한 뒤의 호출은 다시 시도합니다
- `init()` — WASM 모듈 초기화
- `convert(data)` — PPTX 바이트를 HTML로 변환
- `convert_with_options(data, embedImages, includeHidden, slideIndices, scale)`
- `convert_with_metadata(data)` — 변환과 함께 표준 진단 JSON 및 미해석 요소 메타데이터 반환
- `convert_with_options_metadata(data, embedImages, includeHidden, slideIndices, scale)`
- `get_presentation_info(data)` — 타입이 부여된 프레젠테이션 메타데이터
- `get_info(data)` / `get_slide_count(data)` — 하위 호환 헬퍼

메타데이터 결과는 `diagnostics`(순서가 보장된 표준 JSON), `diagnosticsJson`, 그리고 하위 호환용
`unresolvedElements` JSON을 제공합니다.

### 슬라이드 인덱싱

- `convert_slides(data, slides)`는 **0부터 시작하는** 인덱스를 사용합니다.
- `convert_with_options(..., slideIndices)`와 `convert_with_options_metadata(..., slideIndices)`는 **1부터 시작하는** 인덱스를 사용합니다.

### 슬라이드 배율

- `scale`은 필수입니다. 원본 크기를 쓰려면 `1.0`을 전달하세요.
- `2.0` 같은 값은 좌표를 다시 계산하거나 텍스트를 재배치하지 않고 슬라이드 캔버스 전체를 균일하게 확대합니다.
- 함께 제공되는 브라우저 데모는 순서가 보장된 진단을 표시하고, 렌더러가 소유한 동작과 타이밍을 opaque origin 프레임에서 실행하며, 이미지처럼 동작하는 확대를 가용 너비에 맞춰 초기화합니다.

## 범위

이 npm 패키지는 **브라우저 ESM/WASM** 배포판이며 **PPTX만** 변환합니다. 상위 프로젝트는 DOCX,
DOC, XLSX, XLS, PPT, PDF도 변환하지만, 이 형식들은 설치된 LibreOffice와 Poppler 실행 파일에
의존하는 네이티브 파이프라인이 필요하므로 **브라우저 WASM에서는 사용할 수 없습니다**.
[통합 문서 변환](https://github.com/brnyxx/pptx2html-turbo/blob/main/docs/UNIVERSAL_DOCUMENTS.md)
문서를 참고하세요.

`@briank-dev/pptx2html-turbo`는 이전 패키지명입니다. 이전 기간 동안 동일한 API와 릴리스 버전으로
계속 제공됩니다.

## 프로젝트

- 저장소: https://github.com/brnyxx/pptx2html-turbo
- 이슈: https://github.com/brnyxx/pptx2html-turbo/issues
- 데모: https://brnyxx.github.io/pptx2html-turbo/
- 라이선스: MIT
