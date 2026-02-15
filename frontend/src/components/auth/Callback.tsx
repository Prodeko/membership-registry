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
    } else if (error) {
      console.error("Error during OAuth callback:", error);
    }
  }, [data, error]);

  return <div>Loading...</div>;
};

export default Callback;
