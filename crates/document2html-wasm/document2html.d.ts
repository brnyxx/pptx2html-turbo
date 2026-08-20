export type DocumentFormat = "pptx" | "docx" | "doc" | "xlsx" | "xls" | "ppt" | "pdf";
export type RuntimeSupport = "available" | "backend-unavailable";

export interface RuntimeCapability {
  format: DocumentFormat;
  support: RuntimeSupport;
  backend: string | null;
}

export function detect_document_format(
  data: Uint8Array,
  filename?: string | null,
): DocumentFormat;

export function convert_document(
  data: Uint8Array,
  filename?: string | null,
): string;

export function runtime_capabilities_json(): string;
