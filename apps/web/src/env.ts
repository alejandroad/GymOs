import "server-only";
import { z } from "zod";

// The single source of runtime configuration (handoff rule 14).
// Every variable is documented in /.env.example.
// TODO(E0-T4): add BETTER_AUTH_SECRET and BETTER_AUTH_URL.

const serverSchema = z.object({
  NODE_ENV: z.enum(["development", "test", "production"]).default("development"),
  DATABASE_URL: z.url({ protocol: /^postgres(ql)?$/ }),
});

export type Env = z.infer<typeof serverSchema>;

function loadEnv(): Env {
  // Treat empty strings as unset so `FOO=` in a .env file fails validation.
  const raw = Object.fromEntries(
    Object.keys(serverSchema.shape).map((key) => [key, process.env[key] || undefined]),
  );

  // CI builds run without secrets. Code that needs a value still fails when
  // it is used, but importing this module doesn't.
  if (process.env.SKIP_ENV_VALIDATION === "1") {
    return raw as Env;
  }

  const result = serverSchema.safeParse(raw);
  if (!result.success) {
    throw new Error(
      `Invalid environment variables:\n${z.prettifyError(result.error)}\n` +
        "See .env.example for the expected values.",
    );
  }
  return result.data;
}

export const env = loadEnv();
