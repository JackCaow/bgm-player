import { ref } from "vue";
import { useDebounceFn } from "@vueuse/core";
import type { Project, FileItem } from "@/types";

const STORAGE_KEY = "bgm-projects";
const CURRENT_PROJECT_KEY = "bgm-current-project";
const PROJECT_VERSION = "1.0";

export function useProjects() {
  const projects = ref<Project[]>([]);
  const currentProject = ref<Project | null>(null);

  function loadProjects() {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) {
        projects.value = JSON.parse(saved);
      }

      const currentId = localStorage.getItem(CURRENT_PROJECT_KEY);
      if (currentId) {
        const project = projects.value.find(p => p.id === currentId);
        if (project) {
          currentProject.value = project;
        }
      }
    } catch (error) {
      console.error("Failed to load projects:", error);
      projects.value = [];
    }
  }

  function saveProjects() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(projects.value));
    } catch (error) {
      console.error("Failed to save projects:", error);
      throw error;
    }
  }

  function saveProject(project: Project) {
    project.lastModified = Date.now();
    const index = projects.value.findIndex(p => p.id === project.id);

    if (index >= 0) {
      projects.value[index] = project;
    } else {
      projects.value.push(project);
    }

    saveProjects();
  }

  function createProject(name: string, config: Project["config"]): Project {
    const project: Project = {
      id: crypto.randomUUID(),
      name,
      createdAt: Date.now(),
      lastModified: Date.now(),
      version: PROJECT_VERSION,
      config,
      files: [],
      selectedFileId: null,
      stats: {
        totalProcessed: 0,
        totalFailed: 0,
        totalProcessingTime: 0,
      },
    };

    saveProject(project);
    return project;
  }

  function loadProject(projectId: string): Project | null {
    const project = projects.value.find(p => p.id === projectId);
    if (project) {
      currentProject.value = project;
      try {
        localStorage.setItem(CURRENT_PROJECT_KEY, projectId);
      } catch (error) {
        console.error("Failed to save current project:", error);
      }
      return project;
    }
    return null;
  }

  function deleteProject(projectId: string) {
    projects.value = projects.value.filter(p => p.id !== projectId);
    saveProjects();

    if (currentProject.value?.id === projectId) {
      currentProject.value = null;
      try {
        localStorage.removeItem(CURRENT_PROJECT_KEY);
      } catch (error) {
        console.error("Failed to remove current project:", error);
      }
    }
  }

  function renameProject(projectId: string, newName: string) {
    const project = projects.value.find(p => p.id === projectId);
    if (project) {
      project.name = newName;
      project.lastModified = Date.now();
      saveProjects();
    }
  }

  function updateProjectFiles(projectId: string, files: FileItem[], selectedFileId: string | null) {
    const project = projects.value.find(p => p.id === projectId);
    if (project) {
      project.files = files;
      project.selectedFileId = selectedFileId;
      project.lastModified = Date.now();
      saveProjects();
    }
  }

  function updateProjectStats(projectId: string, stats: Partial<Project["stats"]>) {
    const project = projects.value.find(p => p.id === projectId);
    if (project) {
      project.stats = { ...project.stats, ...stats };
      project.lastModified = Date.now();
      saveProjects();
    }
  }

  function updateProjectConfig(projectId: string, config: Partial<Project["config"]>) {
    const project = projects.value.find(p => p.id === projectId);
    if (project) {
      project.config = { ...project.config, ...config };
      project.lastModified = Date.now();
      saveProjects();
    }
  }

  function exportProject(projectId: string): string {
    const project = projects.value.find(p => p.id === projectId);
    if (!project) {
      throw new Error("Project not found");
    }

    return JSON.stringify(project, null, 2);
  }

  function importProject(projectJson: string): Project {
    try {
      const project = JSON.parse(projectJson) as Project;
      project.id = crypto.randomUUID();
      project.createdAt = Date.now();
      project.lastModified = Date.now();

      saveProject(project);
      return project;
    } catch (error) {
      console.error("Failed to import project:", error);
      throw new Error("Invalid project format");
    }
  }

  const autoSave = useDebounceFn((projectId: string, files: FileItem[], selectedFileId: string | null) => {
    updateProjectFiles(projectId, files, selectedFileId);
  }, 3000);

  loadProjects();

  return {
    projects,
    currentProject,
    loadProjects,
    saveProject,
    createProject,
    loadProject,
    deleteProject,
    renameProject,
    updateProjectFiles,
    updateProjectStats,
    updateProjectConfig,
    exportProject,
    importProject,
    autoSave,
  };
}
