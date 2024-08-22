import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider, createBrowserRouter } from "react-router-dom";
import ApplicationForm from "./components/application-form/ApplicationForm";
import Applications from "./components/applications/Applications";
import TargetableRoles from "./components/applications/TargetableRoles";
import Callback from "./components/auth/Callback";
import Error from "./components/Error";
import Layout from "./components/layout/Layout";
import Member from "./components/members/Member";
import Members from "./components/members/Members";
import Roles from "./components/roles/Roles";
import SignupForm from "./components/signup-form/SignupForm";

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
    path: "/applications",
    element: (
      <Layout>
        <Applications />
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
    path: "/application-form",
    element: <ApplicationForm />,
  },
  {
    path: "/signup",
    element: <SignupForm />,
  },
  {
    path: "/auth/callback",
    element: <Callback />,
  }
]);

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  );
}

export default App;
