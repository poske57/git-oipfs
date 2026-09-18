package main

import (
	"errors"
	git "github.com/go-git/go-git/v6"
	"log/slog"
	"os"
	"time"
)

func main() {
	ticker := time.NewTicker(15 * time.Minute)
	defer ticker.Stop()
	for {
		slog.Info("Start periodic reconciliation.")
		err := Reconciliation()
		if err != nil {
			slog.Error("Reconciliation failed.", "err", err.Error())
		}
		<-ticker.C
	}
}

func Reconciliation() error {
	initialize()
	// Check update
	repositoryPath := os.Getenv("OIPFS_REPOSITORY_PATH")
	repo, err := git.PlainOpen(repositoryPath)
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

func cloneIfNotExist() (repo, error) {
	repositoryPath := os.Getenv("OIPFS_REPOSITORY_PATH")

	_, err := git.PlainOpen(repositoryPath)

	if errors.Is(err, git.ErrRepositoryNotExists) {
		_, err := git.PlainClone(repositoryPath, &git.CloneOptions{
			URL:           "https://github.com//example.git",
			ReferenceName: "refs/heads/develop",
			SingleBranch:  true,
			Progress:      os.Stdout,
		})
		if err != nil {
			return err
		}
	}
	return nil
}

type Settings struct {
	repository          *git.Repository
	repositoryUrl       string
	reconciliationCycle int
}

func (s *Settings) loadSettings() error {
	// repository
	repositoryPath := os.Getenv("OIPFS_REPOSITORY_PATH")
	// TODO: print custom error and panic
	if repositoryPath == "" {
		slog.Error("You must set OIPFS_REPOSITORY_PATH")
	}
}
