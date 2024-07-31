import { Link } from "react-router-dom";
import { Button } from "../ui/button";

const Unauthorized = () => {
  return (
    <div className="flex flex-col items-center justify-center text-center h-screen p-10 space-y-8">
      <h1 className="text-4xl">401 unauthorized</h1>
      <p>
        You are not authorized to view this page. Please login again. If the
        issue persists please contact mediakeisari@prodeko.org
      </p>
      <Link
          className="bg-primary text-white px-4 py-2 rounded w-fit text-xl"
          to={`${
            import.meta.env.VITE_API_BASE_URL
          }/auth/login`}
      >
        Login
      </Link>
    </div>
  );
};
``;
export default Unauthorized;
