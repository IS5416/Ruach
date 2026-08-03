/** Error envelope coming over IPC: `{ code, message }` from Rust AppError. */
export interface AppErrorDto {
  code: string;
  message: string;
}

export class RuachError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = "RuachError";
    this.code = code;
  }

  static fromUnknown(e: unknown): RuachError {
    if (typeof e === "object" && e !== null && "code" in e && "message" in e) {
      const dto = e as AppErrorDto;
      return new RuachError(dto.code, dto.message);
    }
    return new RuachError("internal", String(e));
  }
}
