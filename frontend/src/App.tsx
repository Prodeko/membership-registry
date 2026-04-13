import {
  MutationCache,
  QueryCache,
  QueryClient,
  QueryClientProvider,
} from "@tanstack/react-query";
import { RouterProvider, createBrowserRouter } from "react-router-dom";
import { AxiosError } from "axios";
import { toast } from "sonner";
import ApplicationForm from "./components/application-form/ApplicationForm";
import Application from "./components/applications/Application";
import Applications from "./components/applications/Applications";
import TargetableRoles from "./components/applications/targetable-roles/TargetableRoles";
import Callback from "./components/auth/Callback";
import RequireOnboarded from "./components/auth/RequireOnboarded";
import ErrorPage from "./components/Error";
import Onboarding from "./components/onboarding/Onboarding";
import Layout from "./components/layout/Layout";
import Member from "./components/members/Member";
import Members from "./components/members/Members";
import Role from "./components/roles/Role";
import Roles from "./components/roles/Roles";
import AuditLogs from "./components/audit-logs/AuditLogs";
import DataManagement from "./components/data/DataManagement";
import EmailTemplates from "./components/email-templates/EmailTemplates";
import MarketingTags from "./components/marketing-tags/MarketingTags";
import { ThemeProvider } from "./components/theme-provider";
import Success from "./components/application-form/Success";
import PaymentSuccess from "./components/payment/PaymentSuccess";
import UserHome from "./components/home/UserHome";
import ProfileEdit from "./components/profile/ProfileEdit";
import { TooltipProvider } from "./components/ui/tooltip";
import { Toaster } from "./components/ui/sonner";

function getErrorMessage(error: unknown): string {
  if (error instanceof AxiosError) {
    if (error.response?.data && typeof error.response.data === "string") {
      return error.response.data;
    }
    if (error.message) return error.message;
  }
  if (error instanceof Error) return error.message;
  return "An unexpected error occurred";
}

const queryClient = new QueryClient({
  queryCache: new QueryCache({
    onError: (error) => {
      if (error instanceof AxiosError) {
        const status = error.response?.status;
        if (!status || status >= 500) {
          toast.error(getErrorMessage(error));
        }
      } else {
        toast.error(getErrorMessage(error));
      }
    },
  }),
  mutationCache: new MutationCache({
    onError: (error) => {
      toast.error(getErrorMessage(error));
    },
  }),
});

const router = createBrowserRouter([
  {
    path: "/error/:status",
    element: <ErrorPage />,
  },
  {
    path: "/",
    element: (
      <RequireOnboarded>
        <Layout>
          <Members />
        </Layout>
      </RequireOnboarded>
    ),
    errorElement: <ErrorPage />,
  },
  {
    path: "/members",
    element: (
      <RequireOnboarded>
        <Layout>
          <Members />
        </Layout>
      </RequireOnboarded>
    ),
  },
  {
    path: "/members/:id",
    element: (
      <RequireOnboarded>
        <Layout>
          <Member />
        </Layout>
      </RequireOnboarded>
    ),
  },
  {
    path: "/roles",
    element: (
      <RequireOnboarded>
        <Layout>
          <Roles />
        </Layout>
      </RequireOnboarded>
    ),
  },
  {
    path: "/roles/:id",
    element: (
      <RequireOnboarded>
        <Layout>
          <Role />
        </Layout>
      </RequireOnboarded>
    ),
  },
  {
    path: "/applications",
    element: (
      <RequireOnboarded>
        <Layout>
          <Applications />
        </Layout>
      </RequireOnboarded>
    ),
  },
  {
    path: "/applications/:id",
    element: (
      <RequireOnboarded>
        <Layout>
          <Application />
        </Layout>
      </RequireOnboarded>
    ),
  },
  {
    path: "/applications/targetable-roles",
    element: (
      <RequireOnboarded>
        <Layout>
          <TargetableRoles />
        </Layout>
      </RequireOnboarded>
    ),
  },
  {
    path: "/logs",
    element: (
      <RequireOnboarded>
        <Layout>
          <AuditLogs />
        </Layout>
      </RequireOnboarded>
    ),
  },
  {
    path: "/email-templates",
    element: (
      <RequireOnboarded>
        <Layout>
          <EmailTemplates />
        </Layout>
      </RequireOnboarded>
    ),
  },
  {
    path: "/marketing-tags",
    element: (
      <RequireOnboarded>
        <Layout>
          <MarketingTags />
        </Layout>
      </RequireOnboarded>
    ),
  },
  {
    path: "/data",
    element: (
      <RequireOnboarded>
        <Layout>
          <DataManagement />
        </Layout>
      </RequireOnboarded>
    ),
  },
  {
    path: "/home",
    element: (
      <RequireOnboarded>
        <UserHome />
      </RequireOnboarded>
    ),
  },
  {
    path: "/profile/edit",
    element: (
      <RequireOnboarded>
        <ProfileEdit />
      </RequireOnboarded>
    ),
  },
  {
    path: "/apply",
    element: (
      <RequireOnboarded>
        <ApplicationForm />
      </RequireOnboarded>
    ),
  },
  {
    path: "/apply/success",
    element: (
      <RequireOnboarded>
        <Success />
      </RequireOnboarded>
    ),
  },
  {
    path: "/payment/success",
    element: (
      <RequireOnboarded>
        <PaymentSuccess />
      </RequireOnboarded>
    ),
  },
  {
    path: "/onboarding",
    element: <Onboarding />,
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
      <Toaster />
    </ThemeProvider>
  );
}

export default App;
