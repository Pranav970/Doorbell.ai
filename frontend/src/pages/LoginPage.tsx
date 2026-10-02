import { useState, type FormEvent } from 'react';
import { useMutation } from '@tanstack/react-query';
import { Link, useNavigate } from 'react-router-dom';
import { useAuth, isOrgAdmin } from '../context/AuthContext';
import { errorMessage } from '../lib/api';
import { AuthShell } from '../components/Shell';
import { Card, ErrorText } from '../components/Card';
import { Button } from '../components/Button';
import { Field, Input } from '../components/Input';

export function LoginPage() {
  const { login } = useAuth();
  const navigate = useNavigate();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');

  const signIn = useMutation({
    mutationFn: () => login(email, password),
    onSuccess: (me) => navigate(isOrgAdmin(me) ? '/admin' : '/app', { replace: true }),
  });

  const onSubmit = (e: FormEvent) => {
    e.preventDefault();
    signIn.mutate();
  };

  return (
    <AuthShell
      title="Sign in to your gateway"
      subtitle="Use your organization credentials to continue"
      footer={
        <p className="mt-4 text-center text-sm text-muted">
          New here?{' '}
          <Link to="/signup" className="font-medium text-blue-400 hover:text-blue-300 hover:underline">
            Create an account
          </Link>
        </p>
      }
    >
      <Card>
        <form onSubmit={onSubmit} className="flex flex-col gap-4">
          <ErrorText message={errorMessage(signIn.error, 'Could not sign in.')} />
          <Field label="Email">
            <Input
              type="email"
              required
              autoComplete="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
            />
          </Field>
          <Field label="Password">
            <Input
              type="password"
              required
              autoComplete="current-password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
            />
          </Field>
          <Button type="submit" disabled={signIn.isPending} className="mt-1 w-full">
            {signIn.isPending ? 'Signing in…' : 'Sign in'}
          </Button>
        </form>
      </Card>
    </AuthShell>
  );
}
