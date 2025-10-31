// Project feature types

export interface ProjectColor {
  name: string;
  bg: string; // Background color
  border: string; // Border color
  text: string; // Text color
  dot: string; // Solid dot color
}

export interface RecentProject {
  id: string;
  project: string;
  task: string;
  color?: string;
}

export interface QuickProjectSwitcherProps {
  isOpen: boolean;
  onClose: () => void;

  onSelect: (project: RecentProject) => void;
}

export interface ProjectColorMapping {
  [projectName: string]: string;
}

export interface Project {
  id: string;
  name: string;
  description?: string;
  color: string;
  createdAt: Date;
  archived?: boolean;
}
