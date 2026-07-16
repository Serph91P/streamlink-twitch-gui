import { useQueryClient } from "@tanstack/react-query";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useState } from "react";

import type { TwitchBackend } from "../../api/backend";
import type { TwitchLoginChallenge, TwitchSession } from "../../domain/twitch";
import { twitchQueryKeys } from "../../queries/twitch";

interface LoginAttempt {
  challenge: TwitchLoginChallenge;
  expiresAt: number;
}

function verificationUri(value: string): string {
  const url = new URL(value);
  if (url.origin !== "https://www.twitch.tv") {
    throw new Error("Twitch returned an unsupported verification URL");
  }
  return value;
}

export function TwitchAuthPanel({
  backend,
  session,
}: {
  backend: TwitchBackend;
  session: TwitchSession;
}) {
  const queryClient = useQueryClient();
  const [attempt, setAttempt] = useState<LoginAttempt>();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    if (!attempt) return;

    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    const controller = new AbortController();
    const interval = attempt.challenge.pollingIntervalSeconds * 1_000;

    const poll = async () => {
      if (Date.now() >= attempt.expiresAt) {
        setAttempt(undefined);
        setError("Twitch authorization expired");
        return;
      }
      try {
        const updated = await backend.pollTwitchLogin(controller.signal);
        if (!active) return;
        if (updated.status === "authenticated") {
          await queryClient.invalidateQueries({
            queryKey: twitchQueryKeys.all,
            refetchType: "none",
          });
          queryClient.setQueryData(twitchQueryKeys.session, updated);
          setAttempt(undefined);
          setError("");
          return;
        }
        timer = setTimeout(poll, interval);
      } catch (reason) {
        if (!active) return;
        setAttempt(undefined);
        setError(
          reason instanceof Error
            ? reason.message
            : "Could not complete Twitch authorization",
        );
      }
    };

    timer = setTimeout(poll, interval);
    return () => {
      active = false;
      clearTimeout(timer);
      controller.abort();
    };
  }, [attempt, backend, queryClient]);

  async function openVerificationUri(uri: string) {
    try {
      await openUrl(verificationUri(uri));
    } catch (reason) {
      setError(
        reason instanceof Error
          ? reason.message
          : "Could not open the Twitch verification page",
      );
    }
  }

  async function beginLogin() {
    setBusy(true);
    setError("");
    try {
      const challenge = await backend.beginTwitchLogin();
      verificationUri(challenge.verificationUri);
      setAttempt({
        challenge,
        expiresAt: Date.now() + challenge.expiresInSeconds * 1_000,
      });
      await openVerificationUri(challenge.verificationUri);
    } catch (reason) {
      setError(
        reason instanceof Error
          ? reason.message
          : "Could not start Twitch authorization",
      );
    } finally {
      setBusy(false);
    }
  }

  async function signOut() {
    setBusy(true);
    setError("");
    try {
      await backend.signOut();
      queryClient.setQueryData(twitchQueryKeys.session, {
        status: "anonymous",
      });
      queryClient.removeQueries({
        predicate: (query) =>
          query.queryKey[0] === twitchQueryKeys.all[0] &&
          query.queryKey[1] !== "session",
      });
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Could not sign out");
    } finally {
      setBusy(false);
    }
  }

  if (session.status === "authenticated") {
    return (
      <div className="auth-panel">
        <strong>Signed in as {session.user.displayName}</strong>
        <button disabled={busy} onClick={() => void signOut()}>
          Sign out
        </button>
        {error ? <p role="alert">{error}</p> : null}
      </div>
    );
  }

  return (
    <div className="state-panel auth-panel">
      <strong>Sign in with Twitch to browse</strong>
      <p>
        Authorize this app with Twitch to load live channels, categories,
        follows, and search results.
      </p>
      {attempt ? (
        <>
          <p>
            Open <span>{attempt.challenge.verificationUri}</span> and enter:
          </p>
          <code className="device-code">{attempt.challenge.userCode}</code>
          <div className="auth-actions">
            <button
              onClick={() =>
                void openVerificationUri(attempt.challenge.verificationUri)
              }
            >
              Open Twitch
            </button>
            <button
              onClick={() => {
                setAttempt(undefined);
                setError("");
              }}
            >
              Cancel
            </button>
          </div>
          <p role="status">Waiting for Twitch authorization...</p>
        </>
      ) : (
        <button disabled={busy} onClick={() => void beginLogin()}>
          Sign in with Twitch
        </button>
      )}
      {error ? <p role="alert">{error}</p> : null}
    </div>
  );
}
