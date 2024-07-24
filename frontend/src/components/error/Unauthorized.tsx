import { Button } from "../ui/button";

const Unauthorized = () => {
  return (
    <div>
      <h1>Unauthorized</h1>
      <p>You are not authorized to view this page. Please login again. If the issue persists please contact mediakeisari@prodeko.org</p>
      <Button onClick={() => window.location.href = `${import.meta.env.VITE_API_BASE_URL}/auth/login`}>Login</Button>
    </div>
  );
}
``
export default Unauthorized;