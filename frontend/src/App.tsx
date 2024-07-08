import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import Members from './components/members/Members'
import { RouterProvider, createBrowserRouter } from 'react-router-dom'
import Member from './components/members/Member'


const queryClient = new QueryClient()

const router = createBrowserRouter([
  {
    path: '/',
    element: <Members />,
  },
  {
    path: '/members',
    element: <Members />,
  },
  {
    path: '/members/:id',
    element: <Member />,
  },
])

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  )
}

export default App
