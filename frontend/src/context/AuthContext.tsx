import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { api, getAccessToken } from '../lib/api';
import type { Me } from '../lib/types';

interface AuthContextValue {
  me: Me | null;
  loading: boolean;
  refreshMe: () => Promise<Me | null>;
  login: (email: string, password: string) => Promise<Me>;
  signup: (email: string, password: string, displayName: string) => Promise<void>;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [me, setMe] = useState<Me | null>(null);
  const [loading, setLoading] = useState(true);

  const refreshMe = async (): Promise<Me | null> => {
    if (!getAccessToken()) {
      setMe(null);
      return null;
    }
    try {
      const result = await api.me();
      setMe(result);
      return result;
    } catch {
      setMe(null);
      return null;
    }
  };

  useEffect(() => {
    refreshMe().finally(() => setLoading(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const login = async (email: string, password: string): Promise<Me> => {
    await api.login(email, password);
    const result = await refreshMe();
    if (!result) throw new Error('login succeeded but /api/me failed');
    return result;
  };

  const signup = async (email: string, password: string, displayName: string): Promise<void> => {
    await api.signup(email, password, displayName);
  };

  const logout = async (): Promise<void> => {
    await api.logout();
    setMe(null);
  };

  return <AuthContext.Provider value={{ me, loading, refreshMe, login, signup, logout }}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error('useAuth must be used within an AuthProvider');
  return ctx;
}

export function isMasterAdmin(me: Me | null): boolean {
  return !!me?.is_master_admin;
}

/** A master holds no `org_members` row anywhere, so the membership scan
 *  below would send them to the member dashboard — they get the admin
 *  console on the flag instead. */
export function isOrgAdmin(me: Me | null): boolean {
  return isMasterAdmin(me) || !!me?.org_memberships.some((m) => m.role === 'admin');
}

export function primaryAdminOrgId(me: Me | null): string | null {
  return me?.org_memberships.find((m) => m.role === 'admin')?.org_id ?? null;
}

export function approvedTeams(me: Me | null) {
  return me?.team_memberships.filter((m) => m.status === 'approved') ?? [];
}
