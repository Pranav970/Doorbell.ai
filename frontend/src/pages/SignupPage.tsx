import { useState, type FormEvent } from 'react';
import { useMutation } from '@tanstack/react-query';
import { Link, useNavigate } from 'react-router-dom';
import { useAuth } from '../context/AuthContext';
import { api, errorMessage } from '../lib/api';
import type { OrgWithTeams } from '../lib/types';
import { AuthShell } from '../components/Shell';
import { Card, ErrorText, Muted } from '../components/Card';
import { Button } from '../components/Button';
import { Field, Input } from '../components/Input';

type Step = 'form' | 'create-org' | 'pick-team' | 'pending';
type Mode = 'admin' | 'member';

const STEP_TITLES: Record<Step, string> = {
  form: 'Create your account',
  'create-org': 'Name your organization',
  'pick-team': 'Request to join a team',
  pending: 'Request submitted',
};

const MODES: { value: Mode; label: string }[] = [
  { value: 'admin', label: 'Start an organization' },
  { value: 'member', label: 'Join a team' },
];

export function SignupPage() {
  const { signup, login } = useAuth();
  const navigate = useNavigate();

  const [step, setStep] = useState<Step>('form');
  const [mode, setMode] = useState<Mode>('admin');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [orgs, setOrgs] = useState<OrgWithTeams[]>([]);
  const [orgName, setOrgName] = useState('');
  const [selectedTeam, setSelectedTeam] = useState('');
  const [formError, setFormError] = useState<string | null>(null);

  const createAccount = useMutation({
    mutationFn: async () => {
      await signup(email, password, displayName);
      await login(email, password);
      if (mode === 'admin') return null;
      return (await api.listOrgs()).items;
    },
    onSuccess: (items) => {
      if (items === null) {
        setStep('create-org');
      } else {
        setOrgs(items);
        setStep('pick-team');
      }
    },
  });

  const createOrg = useMutation({
    mutationFn: () => api.createOrg(orgName),
    onSuccess: () => navigate('/admin', { replace: true }),
  });

  const requestJoin = useMutation({
    mutationFn: () => api.requestJoin(selectedTeam),
    onSuccess: () => setStep('pending'),
  });

  const submit = (e: FormEvent, run: () => void) => {
    e.preventDefault();
    setFormError(null);
    run();
  };

  return (
    <AuthShell
      title={STEP_TITLES[step]}
      subtitle="Cloud.in unified AI gateway"
      footer={
        step === 'form' && (
          <p className="mt-4 text-center text-sm text-muted">
            Already have an account?{' '}
            <Link to="/login" className="font-medium text-blue-400 hover:text-blue-300 hover:underline">
              Sign in
            </Link>
          </p>
        )
      }
    >
      {step === 'form' && (
        <Card>
          <form onSubmit={(e) => submit(e, createAccount.mutate)} className="flex flex-col gap-4">
            <ErrorText message={errorMessage(createAccount.error, 'Could not create your account.')} />
            <div className="flex gap-1 rounded-lg border border-line bg-raised p-1 text-sm">
              {MODES.map((m) => (
                <button
                  key={m.value}
                  type="button"
                  onClick={() => setMode(m.value)}
                  aria-pressed={mode === m.value}
                  className={`flex-1 whitespace-nowrap rounded-md px-2 py-1.5 text-xs font-medium transition ${
                    mode === m.value ? 'bg-blue-600 text-white' : 'text-muted hover:text-fg'
                  }`}
                >
                  {m.label}
                </button>
              ))}
            </div>
            <Field label="Full name">
              <Input value={displayName} onChange={(e) => setDisplayName(e.target.value)} autoComplete="name" />
            </Field>
            <Field label="Email">
              <Input
                type="email"
                required
                autoComplete="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
              />
            </Field>
            <Field label="Password" hint="At least 8 characters.">
              <Input
                type="password"
                required
                minLength={8}
                autoComplete="new-password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
              />
            </Field>
            <Button type="submit" disabled={createAccount.isPending} className="mt-1 w-full">
              {createAccount.isPending ? 'Creating account…' : 'Continue'}
            </Button>
          </form>
        </Card>
      )}

      {step === 'create-org' && (
        <Card>
          <form onSubmit={(e) => submit(e, createOrg.mutate)} className="flex flex-col gap-4">
            <ErrorText message={errorMessage(createOrg.error, 'Could not create the organization.')} />
            <Field label="Organization name">
              <Input required value={orgName} onChange={(e) => setOrgName(e.target.value)} />
            </Field>
            <Button type="submit" disabled={createOrg.isPending} className="w-full">
              {createOrg.isPending ? 'Creating…' : 'Create organization'}
            </Button>
          </form>
        </Card>
      )}

      {step === 'pick-team' && (
        <Card>
          <form
            onSubmit={(e) =>
              submit(e, () => {
                if (!selectedTeam) {
                  setFormError('Pick a team to request to join.');
                  return;
                }
                requestJoin.mutate();
              })
            }
            className="flex flex-col gap-4"
          >
            <ErrorText message={formError ?? errorMessage(requestJoin.error, 'Could not submit the join request.')} />
            {orgs.length === 0 && <Muted>No organizations exist yet — ask an admin to create one.</Muted>}
            <div className="flex flex-col gap-4">
              {orgs.map((org) => (
                <div key={org.id}>
                  <p className="mb-2 text-xs font-medium uppercase tracking-wider text-muted">{org.name}</p>
                  <div className="flex flex-col gap-1">
                    {org.teams.map((team) => (
                      <label
                        key={team.id}
                        className="flex cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-sm transition hover:bg-blue-500/5"
                      >
                        <input
                          type="radio"
                          name="team"
                          value={team.id}
                          checked={selectedTeam === team.id}
                          onChange={() => setSelectedTeam(team.id)}
                        />
                        {team.name}
                      </label>
                    ))}
                    {org.teams.length === 0 && <p className="px-2 text-xs text-muted">No teams yet.</p>}
                  </div>
                </div>
              ))}
            </div>
            <Button type="submit" disabled={requestJoin.isPending || orgs.length === 0} className="w-full">
              {requestJoin.isPending ? 'Submitting…' : 'Request to join'}
            </Button>
          </form>
        </Card>
      )}

      {step === 'pending' && (
        <Card>
          <Muted>
            Your request has been sent to the team's admin for approval. Once approved you'll be able to add provider
            keys and issue a unified API key.
          </Muted>
          <Link
            to="/app"
            className="mt-4 inline-block text-sm font-medium text-blue-400 hover:text-blue-300 hover:underline"
          >
            Go to your dashboard →
          </Link>
        </Card>
      )}
    </AuthShell>
  );
}
