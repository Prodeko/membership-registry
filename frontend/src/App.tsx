import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider, createBrowserRouter } from "react-router-dom";
import Layout from "./components/layout/Layout";
import Member from "./components/members/Member";
import Members from "./components/members/Members";
import Roles from "./components/roles/Roles";
import TargetableRoles from "./components/applications/TargetableRoles";

const queryClient = new QueryClient();

const router = createBrowserRouter([
  {
    path: "/",
    element: (
      <Layout>
        <Members />
      </Layout>
    ),
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
        <Roles />
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
