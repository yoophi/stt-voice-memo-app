import { AppQueryClientProvider } from "@/app/providers/query-client-provider";
import { HomePage } from "@/pages/home";

export function App() {
  return (
    <AppQueryClientProvider>
      <HomePage />
    </AppQueryClientProvider>
  );
}
