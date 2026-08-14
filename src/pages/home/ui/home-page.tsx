import { APP_IDENTITY } from "@/shared/config/app-identity";
import { FoundationStatus } from "@/widgets/foundation-status";

export function HomePage() {
  return (
    <main className="app-shell">
      <FoundationStatus productName={APP_IDENTITY.productName} />
    </main>
  );
}
