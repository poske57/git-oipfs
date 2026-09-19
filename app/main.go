package main

import (
	"errors"
	"fmt"
	"log/slog"
	"os"
	"time"

	git "github.com/go-git/go-git/v6"
)

func main() {
	settings := &Settings{}
	if err := settings.loadSettings(); err != nil {
		slog.Error("Failed to load settings.", "err", err.Error())
		os.Exit(1)
	}

	ticker := time.NewTicker(time.Duration(settings.reconciliationCycle) * time.Minute)
	defer ticker.Stop()
	for {
		slog.Info("Start periodic reconciliation.")
		err := Reconciliation(settings)
		if err != nil {
			slog.Error("Reconciliation failed.", "err", err.Error())
		}
		<-ticker.C
	}
}

func Reconciliation(s *Settings) error {
	repo, err := cloneIfNotExist(s)
	if err != nil {
		return err
	}

	updated, err := isRepositoryUpdated(repo)
	if err != nil {
		return err
	}

	if !updated {
		slog.Info("There are no updates to the repository.")
		return nil
	}

	// IPFS
	return nil
}

func isRepositoryUpdated(repo *git.Repository) (bool, error) {
	worktree, err := repo.Worktree()
	if err != nil {
		return false, err
	}

	// TODO: Progress をslog.Infoに表示
	err = worktree.Pull(&git.PullOptions{
		RemoteName: "origin",
	})

	if errors.Is(err, git.NoErrAlreadyUpToDate) {
		return false, nil
	}

	if err != nil {
		return false, err
	}

	return true, nil
}

func cloneIfNotExist(s *Settings) (*git.Repository, error) {
	repo, err := git.PlainOpen(s.repositoryPath)
	if errors.Is(err, git.ErrRepositoryNotExists) {
		return git.PlainClone(s.repositoryPath, &git.CloneOptions{
			URL:           s.repositoryUrl,
			SingleBranch:  true,
			ReferenceName: "refs/heads/main",
			Progress:      os.Stdout,
		})
	}
	if err != nil {
		return nil, err
	}
	return repo, nil
}

type Settings struct {
	repositoryPath      string
	repositoryUrl       string
	reconciliationCycle int
}

func (s *Settings) loadSettings() error {
	repositoryPath := os.Getenv("OIPFS_REPOSITORY_PATH")
	if repositoryPath == "" {
		return EnvironmentVariableNotFound{variableName: "OIPFS_REPOSITORY_PATH"}
	}
	s.repositoryPath = repositoryPath

	repositoryUrl := os.Getenv("OIPFS_REPOSITORY_URL")
	if repositoryUrl == "" {
		return EnvironmentVariableNotFound{variableName: "OIPFS_REPOSITORY_URL"}
	}
	s.repositoryUrl = repositoryUrl

	reconciliationCycle := os.Getenv("OIPFS_RECONCILIATION_CYCLE")
	if reconciliationCycle == "" {
		return EnvironmentVariableNotFound{variableName: "OIPFS_RECONCILIATION_CYCLE"}
	}
	if _, err := fmt.Sscanf(reconciliationCycle, "%d", &s.reconciliationCycle); err != nil {
		return fmt.Errorf("invalid OIPFS_RECONCILIATION_CYCLE: %w", err)
	}

	return nil
}

type EnvironmentVariableNotFound struct {
	variableName string
}

func (e EnvironmentVariableNotFound) Error() string {
	return fmt.Sprintf("environment variable not found: %v", e.variableName)
}
