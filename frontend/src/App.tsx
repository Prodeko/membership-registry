import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider, createBrowserRouter } from "react-router-dom";
import Layout from "./components/layout/Layout";
import Member from "./components/members/Member";
import Members from "./components/members/Members";

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
]);

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  );
}

export default App;
