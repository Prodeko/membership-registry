import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider, createBrowserRouter } from "react-router-dom";
import ApplicationForm from "./components/application-form/ApplicationForm";
import Application from "./components/applications/Application";
import Applications from "./components/applications/Applications";
import TargetableRoles from "./components/applications/targetable-roles/TargetableRoles";
import Callback from "./components/auth/Callback";
import Error from "./components/Error";
import Layout from "./components/layout/Layout";
import Member from "./components/members/Member";
import Members from "./components/members/Members";
import Role from "./components/roles/Role";
import Roles from "./components/roles/Roles";
import AuditLogs from "./components/audit-logs/AuditLogs";
import EmailTemplates from "./components/email-templates/EmailTemplates";
import SignupForm from "./components/signup-form/SignupForm";
import { ThemeProvider } from "./components/theme-provider";
import Success from "./components/application-form/Success";
import UserHome from "./components/home/UserHome";
import ProfileEdit from "./components/profile/ProfileEdit";
import { TooltipProvider } from "./components/ui/tooltip";

const queryClient = new QueryClient();

const router = createBrowserRouter([
  {
    path: "/error/:status",
    element: <Error />,
  },
  {
    path: "/",
    element: (
      <Layout>
        <Members />
      </Layout>
    ),
    errorElement: <Error />,
  },
  {
    path: "/members",
    element: (
      <Layout>
        <Members />
      </Layout>
    ),
  },
  {
    path: "/members/:id",
    element: (
      <Layout>
        <Member />
      </Layout>
    ),
  },
  {
    path: "/roles",
    element: (
      <Layout>
        <Roles />
      </Layout>
    ),
  },
  {
    path: "/roles/:id",
    element: (
      <Layout>
        <Role />
      </Layout>
    ),
  },
  {
    path: "/applications",
    element: (
      <Layout>
        <Applications />
      </Layout>
    ),
  },
  {
    path: "/applications/:id",
    element: (
      <Layout>
        <Application />
      </Layout>
    ),
  },
  {
    path: "/applications/targetable-roles",
    element: (
      <Layout>
        <TargetableRoles />
      </Layout>
    ),
  },
  {
    path: "/logs",
    element: (
      <Layout>
        <AuditLogs />
      </Layout>
    ),
  },
  {
    path: "/email-templates",
    element: (
      <Layout>
        <EmailTemplates />
      </Layout>
    ),
  },
  {
    path: "/home",
    element: <UserHome />,
  },
  {
    path: "/profile/edit",
    element: <ProfileEdit />,
  },
  {
    path: "/apply",
    element: <ApplicationForm />,
  },
  {
    path: "/apply/success",
    element: <Success />,
  },
  {
    path: "/signup",
    element: <SignupForm />,
  },
  {
    path: "/auth/callback",
    element: <Callback />,
  },
]);

function App() {
  return (
    <ThemeProvider>
      <QueryClientProvider client={queryClient}>
        <TooltipProvider>
          <RouterProvider router={router} />
        </TooltipProvider>
      </QueryClientProvider>
    </ThemeProvider>
  );
}

export default App;
