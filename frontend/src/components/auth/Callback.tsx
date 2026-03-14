import { useOauthCallback } from "@/lib/api";
import { useEffect } from "react";
import { useLocation } from "react-router-dom";

const Callback = () => {
  const location = useLocation();
  const params = new URLSearchParams(location.search);
  const code = params.get("code")!;
  const state = params.get("state")!;
  const { data, error } = useOauthCallback({ code, state });

  useEffect(() => {
    if (data) {
      window.location.href = data.redirect_to;
    }
  }, [data]);

  if (error) {
    return (
      <div className="flex min-h-screen items-center justify-center">
        <div className="text-center space-y-4">
          <h1 className="text-2xl font-semibold">Authentication failed</h1>
          <p className="text-muted-foreground">{error.message}</p>
          <a
            href={`${import.meta.env.VITE_API_BASE_URL}/auth/login`}
            className="inline-block text-primary underline"
          >
            Try logging in again
          </a>
        </div>
      </div>
    );
  }

  return <div>Loading...</div>;
};

export default Callback;
