import { ReactNode } from "react";
import {
  isRouteErrorResponse,
  Link,
  useParams,
  useRouteError,
} from "react-router-dom";

interface ErrorDetails {
  title: string;
  details: ReactNode;
}

const errorDetails = new Map<number, ErrorDetails>([
  [
    401,
    {
      title: "Unauthorized",
      details: (
        <>
          <p>
            You are not authorized to view this page. Please login again or sign
            up. If the issue persists please contact mediakeisari@prodeko.org
          </p>
          <Link
            className="bg-primary text-white px-4 py-2 rounded w-fit text-xl"
            to={`${import.meta.env.VITE_API_BASE_URL}/auth/login`}
          >
            Login or signup
          </Link>
        </>
      ),
    },
  ],
  [
    403,
    {
      title: "Forbidden",
      details: (
        <div>
          <p>
            You don't have permission to view this page. If you think this is a
            mistake, please contact mediakeisari@prodeko.org
          </p>
        </div>
      ),
    },
  ],
  [
    404,
    {
      title: "Not found",
      details: <div>This page doesn't exist!</div>,
    },
  ],
  [
    503,
    {
      title: "Service unavailable",
      details: <div>Looks like our API is down</div>,
    },
  ],
  [
    418,
    {
      title: "Teapot",
      details: <div>🫖</div>,
    },
  ],
  [
    500,
    {
      title: "Server side error",
      details: <div>Something went wrong at our end 🤷</div>,
    },
  ],
  [
    400,
    {
      title: "Bad request",
      details: <div>Something went wrong with your request 🤷</div>,
    },
  ],
]);

const defaultDetails = {
  title: "Unknown error",
  details: <div>Something went wrong</div>,
};

const Error = () => {
  console.log("Error boundary");
  const error = useRouteError();
  const { status } = useParams();

  const getErrorDetails = () => {
    if (isRouteErrorResponse(error)) {
      return errorDetails.get(error.status) || defaultDetails;
    } else if (!isNaN(Number(status))) {
      return errorDetails.get(Number(status)) || defaultDetails;
    } else {
      return defaultDetails;
    }
  };

  const details = getErrorDetails();

  return (
    <div className="flex flex-col items-center justify-center text-center h-screen p-10 space-y-8">
      <h1 className="text-4xl">{details.title}</h1>
      <div className="space-y-8 h-30 flex flex-col justify-between items-center text-center">
      {details.details}
      </div>
    </div>
  );
};

export default Error;
