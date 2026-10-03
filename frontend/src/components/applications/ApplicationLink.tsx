import { Link, useLocation } from "react-router";
import { ReactNode } from "react";

// Links to an application page, remembering the filtered list URL so the
// application page can return to it.
const ApplicationLink = ({
  id,
  className,
  children,
}: {
  id: string;
  className?: string;
  children: ReactNode;
}) => {
  const location = useLocation();
  return (
    <Link
      to={`/applications/${id}`}
      state={{ from: location.pathname + location.search }}
      className={className}
    >
      {children}
    </Link>
  );
};

export default ApplicationLink;
