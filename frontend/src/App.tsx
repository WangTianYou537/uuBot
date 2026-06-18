import { Navigate, Route, Routes } from "react-router-dom";
import type { ReactNode } from "react";

import LoginPage from "@/pages/LoginPage";
import WordsPage from "@/pages/WordsPage";
import SettingsPage from "@/pages/SettingsPage";
import BotPage from "@/pages/BotPage";
import AdminLoginPage from "@/pages/AdminLoginPage";
import AdminPage from "@/pages/AdminPage";
import { useCurrentUser } from "@/lib/auth";

function RequireUser({ children }: { children: ReactNode }) {
  const { data, isLoading } = useCurrentUser();
  if (isLoading) return <FullPageSpinner />;
  if (!data) return <Navigate to="/" replace />;
  return <>{children}</>;
}

function FullPageSpinner() {
  return (
    <div className="flex min-h-screen items-center justify-center text-muted-foreground">
      加载中…
    </div>
  );
}

export default function App() {
  return (
    <Routes>
      <Route path="/" element={<LoginPage />} />
      <Route
        path="/app"
        element={
          <RequireUser>
            <WordsPage />
          </RequireUser>
        }
      />
      <Route
        path="/settings"
        element={
          <RequireUser>
            <SettingsPage />
          </RequireUser>
        }
      />
      <Route
        path="/bot"
        element={
          <RequireUser>
            <BotPage />
          </RequireUser>
        }
      />
      <Route path="/admin/login" element={<AdminLoginPage />} />
      <Route path="/admin" element={<AdminPage />} />
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
