import { redirect } from "next/navigation";

/** Legacy path — Compatibility Check now lives at /compatibility. */
export default function DiagnosticRedirectPage() {
  redirect("/compatibility");
}
