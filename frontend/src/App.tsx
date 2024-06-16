import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import Members from './components/members/Members'


const queryClient = new QueryClient()

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <Members />
    </QueryClientProvider>
  )
}

export default App
