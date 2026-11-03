"use client";

// Client-rendered app shell. GymOs is local-first: every page renders from
// the on-device database in the browser, never from a server.
export default function Home() {
  return (
    <main>
      <h1>GymOs</h1>
      <p>
        Offline-first lifting analytics. Your data lives on your devices,
        syncs between them if you want, and nobody paywalls your own history.
      </p>
    </main>
  );
}
