/** Tauri rejects with a serialized { code, message }, not an Error instance. */
export function errorMessage(value: unknown): string {
  if (typeof value === "object" && value !== null && "message" in value && typeof value.message === "string") {
    return value.message;
  }
  if (typeof value === "string") return value;
  return "Unknown error";
}
