import { ReactNode } from "react";
import Header from "../header/Header";

const Layout = ({ children }: { children: ReactNode }) => {
  return (
    <div className="flex flex-col h-screen">
      <Header />
      <main className="p-6">{children}</main>
    </div>
  );
};

export default Layout;
