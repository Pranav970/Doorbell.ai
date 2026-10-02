import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom';
import { AuthProvider, isOrgAdmin, useAuth } from './context/AuthContext';
import { ThemeProvider } from './context/ThemeContext';
import { LoginPage } from './pages/LoginPage';
import { SignupPage } from './pages/SignupPage';
import { AdminPage } from './pages/AdminPage';
import { MemberPage } from './pages/MemberPage';
import { LogsPage } from './pages/Logs';
import { LandingPage } from './pages/Landing';

const queryClient = new QueryClient({
  // A 401/403/404 from the control plane won't fix itself on a retry, and
  // retrying just delays showing the error. One retry covers a flaky network.
  defaultOptions: { queries: { retry: 1, refetchOnWindowFocus: false } },
});

/** Full-screen hold while `/api/me` resolves — avoids a flash of the login
 *  page for a user who is already signed in. */
function Splash() {
  return (
    <div className="flex min-h-screen items-center justify-center bg-ink">
      <div className="h-6 w-6 animate-spin rounded-full border-2 border-line border-t-blue-500" role="status">
        <span className="sr-only">Loading</span>
      </div>
    </div>
  );
}

function dashboardFor(me: Parameters<typeof isOrgAdmin>[0]) {
  return isOrgAdmin(me) ? '/admin' : '/app';
}

function RequireAuth({ children, requireAdmin }: { children: React.ReactNode; requireAdmin?: boolean }) {
  const { me, loading } = useAuth();

  if (loading) return <Splash />;
  if (!me) return <Navigate to="/login" replace />;
  if (requireAdmin && !isOrgAdmin(me)) return <Navigate to="/app" replace />;
  return <>{children}</>;
}

/** Login/signup are for signed-out visitors only — an authenticated one goes
 *  straight to whichever dashboard their role earns. */
function RequireAnon({ children }: { children: React.ReactNode }) {
  const { me, loading } = useAuth();

  if (loading) return <Splash />;
  if (me) return <Navigate to={dashboardFor(me)} replace />;
  return <>{children}</>;
}

function IndexRedirect() {
  const { me, loading } = useAuth();
  if (loading) return <Splash />;
  if (!me) return <Navigate to="/login" replace />;
  return <Navigate to={dashboardFor(me)} replace />;
}

function AppRoutes() {
  return (
    <Routes>
      {/* Public marketing page. To make it the front door for signed-out
          visitors, return <LandingPage /> instead of the /login redirect in
          `IndexRedirect` — nothing else needs to change. */}
      <Route path="/welcome" element={<LandingPage />} />
      <Route path="/" element={<IndexRedirect />} />
      <Route
        path="/login"
        element={
          <RequireAnon>
            <LoginPage />
          </RequireAnon>
        }
      />
      <Route
        path="/signup"
        element={
          <RequireAnon>
            <SignupPage />
          </RequireAnon>
        }
      />
      <Route
        path="/admin"
        element={
          <RequireAuth requireAdmin>
            <AdminPage />
          </RequireAuth>
        }
      />
      <Route
        path="/app"
        element={
          <RequireAuth>
            <MemberPage />
          </RequireAuth>
        }
      />
      <Route
        path="/logs"
        element={
          <RequireAuth>
            <LogsPage />
          </RequireAuth>
        }
      />
      {/* Unknown path: re-run the index decision rather than 404 into nothing. */}
      <Route path="*" element={<IndexRedirect />} />
    </Routes>
  );
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ThemeProvider>
        <BrowserRouter>
          <AuthProvider>
            <AppRoutes />
          </AuthProvider>
        </BrowserRouter>
      </ThemeProvider>
    </QueryClientProvider>
  );
}
